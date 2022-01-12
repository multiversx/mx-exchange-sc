#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const DEFAULT_EXTERN_SWAP_GAS_LIMIT: u64 = 50000000;

pub mod amm;
pub mod config;
mod events;
pub mod fee;
mod liquidity_pool;
mod oracle;

use config::State;

type AddLiquidityResultType<BigUint> =
    MultiResult3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

type SwapTokensFixedOutputResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Pair:
    amm::AmmModule
    + fee::FeeModule
    + liquidity_pool::LiquidityPoolModule
    + config::ConfigModule
    + oracle::OracleModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + events::EventsModule
{
    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: ManagedAddress,
        router_owner_address: ManagedAddress,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<()> {
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First token ID is not a valid ESDT identifier"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
            "Second token ID is not a valid ESDT identifier"
        );
        require!(
            first_token_id != second_token_id,
            "Exchange tokens cannot be the same"
        );
        let lp_token_id = self.lp_token_identifier().get();
        require!(
            first_token_id != lp_token_id,
            "First token ID cannot be the same as LP token ID"
        );
        require!(
            second_token_id != lp_token_id,
            "Second token ID cannot be the same as LP token ID"
        );
        self.try_set_fee_percents(total_fee_percent, special_fee_percent)?;

        self.state().set_if_empty(&State::ActiveNoSwaps);
        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.extern_swap_gas_limit()
            .set_if_empty(&DEFAULT_EXTERN_SWAP_GAS_LIMIT);

        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.first_token_id().set(&first_token_id);
        self.second_token_id().set(&second_token_id);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<AddLiquidityResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        self.broadcast_pair_reserves();

        let first_payment = match payments.get(0) {
            Some(p) => p,
            None => return sc_error!("bad payments len"),
        };
        let second_payment = match payments.get(1) {
            Some(p) => p,
            None => return sc_error!("bad payments len"),
        };
        require!(payments.len() == 2, "bad payments len");

        let expected_first_token_id = self.first_token_id().get();
        let expected_second_token_id = self.second_token_id().get();
        require!(
            first_payment.token_identifier == expected_first_token_id,
            "bad first payment"
        );
        require!(
            second_payment.token_identifier == expected_second_token_id,
            "bad second payment"
        );
        require!(first_payment.token_nonce == 0, "non zero first token nonce");
        require!(
            second_payment.token_nonce == 0,
            "non zero second token nonce"
        );

        let first_token_amount_desired = first_payment.amount.clone();
        let second_token_amount_desired = second_payment.amount.clone();
        require!(
            first_token_amount_desired > 0u32,
            "Insufficient first token funds sent"
        );
        require!(
            second_token_amount_desired > 0u32,
            "Insufficient second token funds sent"
        );
        require!(
            first_token_amount_desired >= first_token_amount_min,
            "Input first token desired amount is lower than minimul"
        );
        require!(
            second_token_amount_desired >= second_token_amount_min,
            "Input second token desired amount is lower than minimul"
        );
        self.broadcast_pair_reserves();

        let old_k = self.calculate_k_for_reserves();
        let (first_token_amount, second_token_amount) = self.calculate_optimal_amounts(
            first_token_amount_desired.clone(),
            second_token_amount_desired.clone(),
            first_token_amount_min,
            second_token_amount_min,
        )?;

        let liquidity =
            self.pool_add_liquidity(first_token_amount.clone(), second_token_amount.clone())?;
        let first_token_unused = &first_token_amount_desired - &first_token_amount;
        let second_token_unused = &second_token_amount_desired - &second_token_amount;

        // Once liquidity has been added, the new K should always be greater than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&old_k, &new_k)?;

        let lp_token_id = self.lp_token_identifier().get();
        self.mint_tokens(&lp_token_id, &liquidity);

        let mut payments = ManagedVec::new();
        payments.push(self.create_payment(&lp_token_id, 0, &liquidity));
        payments.push(self.create_payment(&expected_first_token_id, 0, &first_token_unused));
        payments.push(self.create_payment(&expected_second_token_id, 0, &second_token_unused));

        let caller = self.blockchain().get_caller();
        self.send_multiple_tokens(&caller, &payments, opt_accept_funds_func)?;

        self.broadcast_pair_reserves();
        self.emit_add_liquidity_event(
            &caller,
            &expected_first_token_id,
            &first_token_amount,
            &expected_second_token_id,
            &second_token_amount,
            &lp_token_id,
            &liquidity,
            &self.get_total_lp_token_supply(),
            &self.pair_reserve(&expected_first_token_id).get(),
            &self.pair_reserve(&expected_second_token_id).get(),
        );
        Ok(MultiResult3::from((
            self.create_payment(&lp_token_id, 0, &liquidity),
            self.create_payment(&expected_first_token_id, 0, &first_token_amount),
            self.create_payment(&expected_second_token_id, 0, &second_token_amount),
        )))
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] liquidity: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<RemoveLiquidityResultType<Self::Api>> {
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.lp_token_identifier().get();
        require!(token_id == lp_token_id, "Wrong liquidity token");
        self.broadcast_pair_reserves();

        let old_k = self.calculate_k_for_reserves();
        let (first_token_amount, second_token_amount) = self.pool_remove_liquidity(
            liquidity.clone(),
            first_token_amount_min,
            second_token_amount_min,
        )?;

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        // Once liquidity has been removed, the new K should always be lesser than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&new_k, &old_k)?;

        let mut payments = ManagedVec::new();
        payments.push(self.create_payment(&first_token_id, 0, &first_token_amount));
        payments.push(self.create_payment(&second_token_id, 0, &second_token_amount));
        self.send_multiple_tokens(&caller, &payments, opt_accept_funds_func)?;

        self.burn_tokens(&token_id, &liquidity);

        self.emit_remove_liquidity_event(
            &caller,
            &first_token_id,
            &first_token_amount,
            &second_token_id,
            &second_token_amount,
            &lp_token_id,
            &liquidity,
            &self.get_total_lp_token_supply(),
            &self.pair_reserve(&first_token_id).get(),
            &self.pair_reserve(&second_token_id).get(),
        );
        Ok(MultiResult2::from((
            self.create_payment(&first_token_id, 0, &first_token_amount),
            self.create_payment(&second_token_id, 0, &second_token_amount),
        )))
    }

    #[payable("*")]
    #[endpoint(removeLiquidityAndBuyBackAndBurnToken)]
    fn remove_liquidity_and_burn_token(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: BigUint,
        token_to_buyback_and_burn: TokenIdentifier,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller)?;

        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );
        require!(
            token_in == self.lp_token_identifier().get(),
            "Wrong liquidity token"
        );

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        let first_token_min_amount = BigUint::from(1u64);
        let second_token_min_amount = BigUint::from(1u64);
        let (first_token_amount, second_token_amount) = self.pool_remove_liquidity(
            amount_in.clone(),
            first_token_min_amount,
            second_token_min_amount,
        )?;

        let dest_address = ManagedAddress::zero();
        self.send_fee_slice(
            &first_token_id,
            &first_token_amount,
            &dest_address,
            &token_to_buyback_and_burn,
            &first_token_id,
            &second_token_id,
        );
        self.send_fee_slice(
            &second_token_id,
            &second_token_amount,
            &dest_address,
            &token_to_buyback_and_burn,
            &first_token_id,
            &second_token_id,
        );
        self.burn_tokens(&token_in, &amount_in);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: BigUint,
        token_out: TokenIdentifier,
        destination_address: ManagedAddress,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller)?;

        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(token_in != token_out, "Cannot swap same token");
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Invalid token out"
        );
        self.broadcast_pair_reserves();

        let old_k = self.calculate_k_for_virtual_reserves(&token_in);

        let amount_out =
            self.swap_safe_no_fee(&first_token_id, &second_token_id, &token_in, &amount_in);
        require!(amount_out > 0, "Zero output");

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_virtual_reserves(&token_in);
        self.validate_k_invariant(&old_k, &new_k)?;

        self.send_fee_or_burn_on_zero_address(&token_out, &amount_out, &destination_address);

        self.emit_swap_no_fee_and_forward_event(
            &caller,
            &token_in,
            &amount_in,
            &token_out,
            &amount_out,
            &destination_address,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: BigUint,
        token_out: TokenIdentifier,
        amount_out_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<SwapTokensFixedInputResultType<Self::Api>> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in > 0u32, "Invalid amount_in");
        require!(token_in != token_out, "Swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Invalid token out"
        );
        self.broadcast_pair_reserves();
        self.update_virtual_reserves_on_block_change();
        let old_k = self.calculate_k_for_virtual_reserves(&token_in);

        let mut reserve_token_out = self.pair_virtual_reserve(&token_in, &token_out).get();
        require!(
            reserve_token_out > amount_out_min,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_virtual_reserve(&token_in, &token_in).get();
        let amount_out_optimal =
            self.get_amount_out(&amount_in, &reserve_token_in, &reserve_token_out);
        require!(
            amount_out_optimal >= amount_out_min,
            "Computed amount out lesser than minimum amount out"
        );
        require!(
            reserve_token_out > amount_out_optimal,
            "Insufficient amount out reserve"
        );
        require!(amount_out_optimal != 0u32, "Optimal value is zero");

        let caller = self.blockchain().get_caller();

        let mut fee_amount = BigUint::zero();
        let mut amount_in_after_fee = amount_in.clone();
        if self.is_fee_enabled() {
            fee_amount = self.get_special_fee_from_input(&amount_in);
            amount_in_after_fee -= &fee_amount;
        }

        reserve_token_in += &amount_in_after_fee;
        reserve_token_out -= &amount_out_optimal;
        self.set_virtual_reserves(
            &token_in,
            &token_in,
            &token_out,
            &reserve_token_in,
            &reserve_token_out,
        );
        self.try_decrease_token_reserve(&token_out, &amount_out_optimal)?;
        self.increase_token_reserve(&token_in, &amount_in_after_fee);

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_virtual_reserves(&token_in);
        self.validate_k_invariant(&old_k, &new_k)?;

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_fee_enabled() {
            self.send_fee(&token_in, &fee_amount);
        }
        self.transfer_execute_custom(
            &caller,
            &token_out,
            0,
            &amount_out_optimal,
            opt_accept_funds_func,
        )?;

        self.emit_swap_event(
            &caller,
            &token_in,
            &amount_in,
            &token_out,
            &amount_out_optimal,
            &fee_amount,
            &reserve_token_in,
            &reserve_token_out,
        );
        Ok(self.create_payment(&token_out, 0, &amount_out_optimal))
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in_max: BigUint,
        token_out: TokenIdentifier,
        amount_out: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<SwapTokensFixedOutputResultType<Self::Api>> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in_max > 0u32, "Invalid amount_in");
        require!(token_in != token_out, "Invalid swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Invalid token out"
        );
        require!(amount_out != 0u32, "Desired amount out cannot be zero");
        self.broadcast_pair_reserves();
        self.update_virtual_reserves_on_block_change();
        let old_k = self.calculate_k_for_virtual_reserves(&token_in);

        let mut reserve_token_out = self.pair_virtual_reserve(&token_in, &token_out).get();
        require!(
            reserve_token_out > amount_out,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_virtual_reserve(&token_in, &token_in).get();
        let amount_in_optimal =
            self.get_amount_in(&amount_out, &reserve_token_in, &reserve_token_out);
        require!(
            amount_in_optimal <= amount_in_max,
            "Computed amount in greater than maximum amount in"
        );

        let caller = self.blockchain().get_caller();
        let residuum = &amount_in_max - &amount_in_optimal;

        let mut fee_amount = BigUint::zero();
        let mut amount_in_optimal_after_fee = amount_in_optimal.clone();
        if self.is_fee_enabled() {
            fee_amount = self.get_special_fee_from_input(&amount_in_optimal);
            amount_in_optimal_after_fee -= &fee_amount;
        }

        reserve_token_in += &amount_in_optimal_after_fee;
        reserve_token_out -= &amount_out;
        self.set_virtual_reserves(
            &token_in,
            &token_in,
            &token_out,
            &reserve_token_in,
            &reserve_token_out,
        );
        self.try_decrease_token_reserve(&token_out, &amount_out)?;
        self.increase_token_reserve(&token_in, &amount_in_optimal_after_fee);

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_virtual_reserves(&token_in);
        self.validate_k_invariant(&old_k, &new_k)?;

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_fee_enabled() {
            self.send_fee(&token_in, &fee_amount);
        }

        let mut payments = ManagedVec::new();
        payments.push(self.create_payment(&token_out, 0, &amount_out));
        payments.push(self.create_payment(&token_in, 0, &residuum));
        self.send_multiple_tokens(&caller, &payments, opt_accept_funds_func)?;

        self.emit_swap_event(
            &caller,
            &token_in,
            &amount_in_optimal,
            &token_out,
            &amount_out,
            &fee_amount,
            &reserve_token_in,
            &reserve_token_out,
        );
        Ok(MultiResult2::from((
            self.create_payment(&token_out, 0, &amount_out),
            self.create_payment(&token_in, 0, &residuum),
        )))
    }

    fn broadcast_pair_reserves(&self) {
        self.update_price_record(
            &self.pair_reserve(&self.first_token_id().get()).get(),
            &self.pair_reserve(&self.second_token_id().get()).get(),
        )
    }

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        require!(self.lp_token_identifier().is_empty(), "LP token not empty");
        require!(
            token_identifier != self.first_token_id().get()
                && token_identifier != self.second_token_id().get(),
            "LP token should differ from the exchange tokens"
        );
        require!(
            token_identifier.is_valid_esdt_identifier(),
            "Provided identifier is not a valid ESDT identifier"
        );

        self.lp_token_identifier().set(&token_identifier);

        Ok(())
    }

    #[inline]
    fn validate_k_invariant(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower <= greater, "K invariant failed");
        Ok(())
    }

    #[inline]
    fn validate_k_invariant_strict(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower < greater, "K invariant failed");
        Ok(())
    }

    #[view(getTokensForGivenPosition)]
    fn get_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        self.get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(&self) -> MultiResult3<BigUint, BigUint, BigUint> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let total_supply = self.get_total_lp_token_supply();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out_view(
        &self,
        token_in: TokenIdentifier,
        amount_in: BigUint,
    ) -> SCResult<BigUint> {
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let (first_token_reserve, second_token_reserve) =
            self.get_reserves_for_current_block(&token_in, &first_token_id, &second_token_id);

        if token_in == first_token_id {
            require!(second_token_reserve > 0, "Zero reserves for second token");
            let amount_out =
                self.get_amount_out(&amount_in, &first_token_reserve, &second_token_reserve);
            require!(
                second_token_reserve > amount_out,
                "Not enough reserves for second token"
            );
            Ok(amount_out)
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0, "Zero reserves for first token");
            let amount_out =
                self.get_amount_out(&amount_in, &second_token_reserve, &first_token_reserve);
            require!(
                first_token_reserve > amount_out,
                "Not enough reserves first token"
            );
            Ok(amount_out)
        } else {
            sc_error!("Not a known token")
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in_view(
        &self,
        token_wanted: TokenIdentifier,
        amount_wanted: BigUint,
    ) -> SCResult<BigUint> {
        require!(amount_wanted > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        if token_wanted == first_token_id {
            let (first_token_reserve, second_token_reserve) = self.get_reserves_for_current_block(
                &second_token_id,
                &first_token_id,
                &second_token_id,
            );
            require!(
                first_token_reserve > amount_wanted,
                "Not enough reserves for first token"
            );
            let amount_in =
                self.get_amount_in(&amount_wanted, &second_token_reserve, &first_token_reserve);
            Ok(amount_in)
        } else if token_wanted == second_token_id {
            let (first_token_reserve, second_token_reserve) = self.get_reserves_for_current_block(
                &first_token_id,
                &first_token_id,
                &second_token_id,
            );
            require!(
                second_token_reserve > amount_wanted,
                "Not enough reserves for second token"
            );
            let amount_in =
                self.get_amount_in(&amount_wanted, &first_token_reserve, &second_token_reserve);
            Ok(amount_in)
        } else {
            sc_error!("Not a known token")
        }
    }

    #[view(getEquivalent)]
    fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> SCResult<BigUint> {
        require!(amount_in > 0, "Zero input");
        let zero = BigUint::zero();

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if first_token_reserve == 0 || second_token_reserve == 0 {
            return Ok(zero);
        }

        if token_in == first_token_id {
            Ok(self.quote(&amount_in, &first_token_reserve, &second_token_reserve))
        } else if token_in == second_token_id {
            Ok(self.quote(&amount_in, &second_token_reserve, &first_token_reserve))
        } else {
            sc_error!("Not a known token")
        }
    }

    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active || state == State::ActiveNoSwaps
    }

    #[inline]
    fn can_swap(&self) -> bool {
        self.state().get() == State::Active
    }
}
