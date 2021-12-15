#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const DEFAULT_EXTERN_SWAP_GAS_LIMIT: u64 = 50000000;

mod amm;
pub mod config;
mod contexts;
mod errors;
mod events;
pub mod fee;
mod liquidity_pool;
pub mod validation;

use crate::errors::*;
use config::State;
use contexts::base::*;
use contexts::ctx_helper;

type AddLiquidityResultType<BigUint> =
    MultiResult3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

type SwapTokensFixedOutputResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Pair<ContractReader>:
    amm::AmmModule
    + fee::FeeModule
    + liquidity_pool::LiquidityPoolModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + events::EventsModule
    + validation::ValidationModule
    + ctx_helper::CtxHelper
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
    ) {
        self.assert(first_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        self.assert(second_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        self.assert(first_token_id != second_token_id, ERROR_SAME_TOKENS);
        let lp_token_id = self.lp_token_identifier().get();
        self.assert(first_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        self.assert(second_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        self.set_fee_percents(total_fee_percent, special_fee_percent);

        self.state().set(&State::Inactive);
        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.extern_swap_gas_limit()
            .set_if_empty(&DEFAULT_EXTERN_SWAP_GAS_LIMIT);

        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.first_token_id().set(&first_token_id);
        self.second_token_id().set(&second_token_id);
    }

    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> AddLiquidityResultType<Self::Api> {
        let mut context = self.new_add_liquidity_context(
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        );
        self.assert(
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        self.assert(
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        self.assert(
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_state(&mut context);
        self.assert(
            self.is_state_active(context.get_contract_state()),
            ERROR_NOT_ACTIVE,
        );

        self.load_lp_token_id(&mut context);
        self.assert(
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );

        self.load_pool_token_ids(&mut context);
        self.assert(
            context.payment_tokens_match_pool_tokens(),
            ERROR_BAD_PAYMENT_TOKENS,
        );

        self.load_pool_reserves(&mut context);
        self.load_lp_token_supply(&mut context);
        self.load_initial_k(&mut context);

        self.calculate_optimal_amounts(&mut context);
        self.pool_add_liquidity(&mut context);

        let new_k = self.calculate_k(&context);
        self.assert(context.get_initial_k() <= &new_k, ERROR_K_INVARIANT_FAILED);

        let lpt = context.get_lp_token_id();
        let liq_added = context.get_liquidity_added();
        self.send().esdt_local_mint(lpt, 0, liq_added);
        self.commit_changes(&context);

        self.construct_add_liquidity_output_payments(&mut context);
        self.execute_output_payments(&context);
        self.emit_add_liquidity_event(&context);

        self.construct_and_get_add_liquidity_output_results(&context)
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] liquidity: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> RemoveLiquidityResultType<Self::Api> {
        let mut context = self.new_remove_liquidity_context(
            &token_id,
            nonce,
            &liquidity,
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        );
        self.assert(
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        self.assert(
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        self.assert(
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_state(&mut context);
        self.assert(
            self.is_state_active(context.get_contract_state()),
            ERROR_NOT_ACTIVE,
        );

        self.load_lp_token_id(&mut context);
        self.assert(
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );
        self.assert(
            context.get_lp_token_id() == &context.get_lp_token_payment().token_identifier,
            ERROR_BAD_PAYMENT_TOKENS,
        );

        self.load_pool_token_ids(&mut context);
        self.load_pool_reserves(&mut context);
        self.load_lp_token_supply(&mut context);
        self.load_initial_k(&mut context);

        self.pool_remove_liquidity(&mut context);

        let new_k = self.calculate_k(&context);
        self.assert(&new_k <= context.get_initial_k(), ERROR_K_INVARIANT_FAILED);

        let lpt = context.get_lp_token_id();
        let liq_removed = &context.get_lp_token_payment().amount;
        self.send().esdt_local_burn(lpt, 0, liq_removed);
        self.commit_changes(&context);

        self.construct_remove_liquidity_output_payments(&mut context);
        self.execute_output_payments(&context);
        self.emit_remove_liquidity_event(&context);

        self.construct_and_get_remove_liquidity_output_results(&context)
    }

    #[payable("*")]
    #[endpoint(removeLiquidityAndBuyBackAndBurnToken)]
    fn remove_liquidity_and_burn_token(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] amount_in: BigUint,
        token_to_buyback_and_burn: TokenIdentifier,
    ) {
        let mut context = self.new_remove_liquidity_context(
            &token_in,
            nonce,
            &amount_in,
            BigUint::from(1u64),
            BigUint::from(1u64),
            OptionalArg::None,
        );
        self.require_whitelisted(context.get_caller()).unwrap();

        self.assert(
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        self.assert(
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        self.assert(
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_lp_token_id(&mut context);
        self.assert(
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );
        self.assert(
            context.get_lp_token_id() == &context.get_lp_token_payment().token_identifier,
            ERROR_BAD_PAYMENT_TOKENS,
        );

        self.load_pool_token_ids(&mut context);
        self.load_pool_reserves(&mut context);
        self.load_lp_token_supply(&mut context);

        self.pool_remove_liquidity(&mut context);

        self.send().esdt_local_burn(&token_in, 0, &amount_in);
        self.lp_token_supply().update(|x| *x -= &amount_in);
        self.commit_changes(&context);

        let dest_address = ManagedAddress::zero();
        self.send_fee_slice(
            context.get_first_token_id(),
            context.get_first_token_amount_removed(),
            &dest_address,
            &token_to_buyback_and_burn,
            context.get_first_token_id(),
            context.get_second_token_id(),
        );
        self.send_fee_slice(
            context.get_second_token_id(),
            context.get_second_token_amount_removed(),
            &dest_address,
            &token_to_buyback_and_burn,
            context.get_first_token_id(),
            context.get_second_token_id(),
        );
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

        let old_k = self.calculate_k_for_reserves();

        let amount_out =
            self.swap_safe_no_fee(&first_token_id, &second_token_id, &token_in, &amount_in);
        require!(amount_out > 0, "Zero output");

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant(&old_k, &new_k)?;

        self.burn_fees(&token_out, &amount_out);

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
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out_min,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
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
        self.update_reserves(&reserve_token_in, &reserve_token_out, &token_in, &token_out);

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
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
            &opt_accept_funds_func,
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
        require!(amount_in_max > 0, "Invalid amount_in");
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
        require!(amount_out != 0, "Desired amount out cannot be zero");
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
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
        self.update_reserves(&reserve_token_in, &reserve_token_out, &token_in, &token_out);

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant(&old_k, &new_k)?;

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_fee_enabled() {
            self.send_fee(&token_in, &fee_amount);
        }

        let mut payments = ManagedVec::new();
        payments.push(self.create_payment(&token_out, 0, &amount_out));
        payments.push(self.create_payment(&token_in, 0, &residuum));
        self.send_multiple_tokens_if_not_zero(&caller, &payments, &opt_accept_funds_func)?;

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

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        self.require_permissions();
        require!(self.lp_token_identifier().is_empty(), "LP token not empty");
        require!(
            token_identifier != self.first_token_id().get()
                && token_identifier != self.second_token_id().get(),
            "LP token should differ from the exchange tokens"
        );
        require!(
            token_identifier.is_esdt(),
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
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

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
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            require!(
                first_token_reserve > amount_wanted,
                "Not enough reserves for first token"
            );
            let amount_in =
                self.get_amount_in(&amount_wanted, &second_token_reserve, &first_token_reserve);
            Ok(amount_in)
        } else if token_wanted == second_token_id {
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
    fn is_state_active(&self, state: &State) -> bool {
        state == &State::Active || state == &State::ActiveNoSwaps
    }

    #[inline]
    fn can_swap(&self) -> bool {
        self.state().get() == State::Active
    }
}
