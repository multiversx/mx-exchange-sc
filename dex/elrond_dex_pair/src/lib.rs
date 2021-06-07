#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const DEFAULT_EXTERN_SWAP_GAS_LIMIT: u64 = 50000000;

mod amm;
mod config;
mod fee;
mod liquidity_pool;

use config::*;
use dex_common::*;

type AddLiquidityResultType<BigUint> = MultiResult3<
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, FftTokenAmountPair<BigUint>>;

#[elrond_wasm_derive::contract]
pub trait Pair:
    amm::AmmModule + fee::FeeModule + liquidity_pool::LiquidityPoolModule + config::ConfigModule
{
    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: Address,
        router_owner_address: Address,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<()> {
        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.first_token_id().set(&first_token_id);
        self.second_token_id().set(&second_token_id);
        require!(
            total_fee_percent >= special_fee_percent && total_fee_percent < 100_000,
            "Bad percents"
        );
        self.total_fee_percent().set(&total_fee_percent);
        self.special_fee_percent().set(&special_fee_percent);
        self.state().set(&State::ActiveNoSwaps);
        self.transfer_exec_gas_limit()
            .set(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.extern_swap_gas_limit()
            .set(&DEFAULT_EXTERN_SWAP_GAS_LIMIT);
        Ok(())
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Active);
        Ok(())
    }

    #[endpoint(setStateActiveNoSwaps)]
    fn set_state_active_no_swaps(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::ActiveNoSwaps);
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn acceptEsdtPayment(
        &self,
        #[payment_token] token: TokenIdentifier,
        #[payment_amount] payment: Self::BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            self.call_value().esdt_token_nonce() == 0,
            "Only fungible tokens are accepted in liquidity pools"
        );
        require!(
            payment > 0,
            "PAIR: Funds transfer must be a positive number"
        );
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token == first_token_id || token == second_token_id,
            "Pair: Invalid token"
        );

        let caller = self.blockchain().get_caller();
        let mut temporary_funds = self.temporary_funds(&caller, &token).get();
        temporary_funds += payment;
        self.temporary_funds(&caller, &token).set(&temporary_funds);

        Ok(())
    }

    #[endpoint]
    fn addLiquidity(
        &self,
        first_token_amount_desired: Self::BigUint,
        second_token_amount_desired: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<AddLiquidityResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
        require!(
            first_token_amount_desired > 0,
            "Pair: insufficient first token funds sent"
        );
        require!(
            second_token_amount_desired > 0,
            "Pair: insufficient second token funds sent"
        );
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );

        let caller = self.blockchain().get_caller();
        let expected_first_token_id = self.first_token_id().get();
        let expected_second_token_id = self.second_token_id().get();
        let temporary_first_token_amount = self
            .temporary_funds(&caller, &expected_first_token_id)
            .get();
        let temporary_second_token_amount = self
            .temporary_funds(&caller, &expected_second_token_id)
            .get();

        require!(
            temporary_first_token_amount > 0,
            "Pair: no available first token funds"
        );
        require!(
            temporary_second_token_amount > 0,
            "Pair: no available second token funds"
        );
        require!(
            first_token_amount_desired <= temporary_first_token_amount,
            "Pair: insufficient first token funds to add"
        );
        require!(
            second_token_amount_desired <= temporary_second_token_amount,
            "Pair: insufficient second token funds to add"
        );

        let old_k = self.calculate_k_for_reserves();
        let (first_token_amount, second_token_amount) = self.calculate_optimal_amounts(
            first_token_amount_desired,
            second_token_amount_desired,
            first_token_amount_min,
            second_token_amount_min,
        )?;

        let liquidity =
            self.add_liquidity(first_token_amount.clone(), second_token_amount.clone())?;

        let caller = &self.blockchain().get_caller();
        let temporary_first_token_unused =
            temporary_first_token_amount - first_token_amount.clone();
        let temporary_second_token_unused =
            temporary_second_token_amount - second_token_amount.clone();
        self.temporary_funds(caller, &expected_first_token_id)
            .clear();
        self.temporary_funds(caller, &expected_second_token_id)
            .clear();

        // Once liquidity has been added, the new K should always be greater than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&old_k, &new_k)?;

        let lp_token_id = self.lp_token_identifier().get();
        self.send().esdt_local_mint(&lp_token_id, &liquidity);

        self.send_tokens(&lp_token_id, &liquidity, caller, &opt_accept_funds_func)?;
        self.send_tokens(
            &expected_first_token_id,
            &temporary_first_token_unused,
            caller,
            &opt_accept_funds_func,
        )?;
        self.send_tokens(
            &expected_second_token_id,
            &temporary_second_token_unused,
            caller,
            &opt_accept_funds_func,
        )?;

        Ok((
            FftTokenAmountPair {
                token_id: lp_token_id,
                amount: liquidity,
            },
            FftTokenAmountPair {
                token_id: expected_first_token_id,
                amount: first_token_amount,
            },
            FftTokenAmountPair {
                token_id: expected_second_token_id,
                amount: second_token_amount,
            },
        )
            .into())
    }

    fn reclaim_temporary_token(
        &self,
        caller: &Address,
        token: &TokenIdentifier,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let amount = self.temporary_funds(caller, token).get();
        self.temporary_funds(caller, token).clear();
        self.send_tokens(token, &amount, caller, opt_accept_funds_func)?;
        Ok(())
    }

    #[endpoint(reclaimTemporaryFunds)]
    fn reclaim_temporary_funds(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        self.reclaim_temporary_token(&caller, &first_token_id, &opt_accept_funds_func)?;
        self.reclaim_temporary_token(&caller, &second_token_id, &opt_accept_funds_func)?;

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn removeLiquidity(
        &self,
        #[payment_token] liquidity_token: TokenIdentifier,
        #[payment_amount] liquidity: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<RemoveLiquidityResultType<Self::BigUint>> {
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );

        let caller = self.blockchain().get_caller();
        require!(
            liquidity_token == self.lp_token_identifier().get(),
            "Pair: wrong liquidity token"
        );

        let old_k = self.calculate_k_for_reserves();
        let (first_token_amount, second_token_amount) = self.remove_liquidity(
            liquidity.clone(),
            first_token_amount_min,
            second_token_amount_min,
        )?;

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        // Once liquidity has been removed, the new K should always be lesser than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&new_k, &old_k)?;

        self.send_tokens(
            &first_token_id,
            &first_token_amount,
            &caller,
            &opt_accept_funds_func,
        )?;
        self.send_tokens(
            &second_token_id,
            &second_token_amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        self.send().esdt_local_burn(&liquidity_token, &liquidity);

        Ok((
            FftTokenAmountPair {
                token_id: first_token_id,
                amount: first_token_amount,
            },
            FftTokenAmountPair {
                token_id: second_token_id,
                amount: second_token_amount,
            },
        )
            .into())
    }

    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: Self::BigUint,
        token_out: TokenIdentifier,
        destination_address: Address,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.whitelist().contains(&caller), "Not whitelisted");
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

        self.send_fee_or_burn_on_zero_address(&token_out, &amount_out, &destination_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out_min: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in > 0, "Invalid amount_in");
        require!(token_in != token_out, "Swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
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
        require!(amount_out_optimal != 0, "Optimal value is zero");

        let caller = self.blockchain().get_caller();

        let mut fee_amount = Self::BigUint::zero();
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
            self.send_fee(&token_in, fee_amount);
        }
        self.send_tokens(
            &token_out,
            &amount_out_optimal,
            &caller,
            &opt_accept_funds_func,
        )?;

        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in_max: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in_max > 0, "Invalid amount_in");
        require!(token_in != token_out, "Invalid swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
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

        let mut fee_amount = Self::BigUint::zero();
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
            self.send_fee(&token_in, fee_amount);
        }

        self.send_tokens(&token_out, &amount_out, &caller, &opt_accept_funds_func)?;
        self.send_tokens(&token_in, &residuum, &caller, &opt_accept_funds_func)?;

        Ok(())
    }

    fn send_tokens(
        &self,
        token: &TokenIdentifier,
        amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        if amount > &0 {
            let (function, gas_limit) = match opt_accept_funds_func {
                OptionalArg::Some(accept_funds_func) => (
                    accept_funds_func.as_slice(),
                    self.transfer_exec_gas_limit().get(),
                ),
                OptionalArg::None => {
                    let no_func: &[u8] = &[];
                    (no_func, 0u64)
                }
            };

            let result = self.send().direct_esdt_execute(
                destination,
                token,
                amount,
                gas_limit,
                function,
                &ArgBuffer::new(),
            );

            match result {
                Result::Ok(_) => Ok(()),
                Result::Err(_) => {
                    sc_error!("Direct esdt nft execute failed")
                }
            }
        } else {
            Ok(())
        }
    }

    #[endpoint]
    fn setLpTokenIdentifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        require!(self.lp_token_identifier().is_empty(), "LP token not empty");
        self.lp_token_identifier().set(&token_identifier);

        Ok(())
    }

    #[endpoint]
    fn setFeeOn(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        self.set_fee_on(enabled, fee_to_address, fee_token)
    }

    #[inline]
    fn validate_k_invariant(&self, lower: &Self::BigUint, greater: &Self::BigUint) -> SCResult<()> {
        require!(lower <= greater, "K invariant failed");
        Ok(())
    }

    #[inline]
    fn validate_k_invariant_strict(
        &self,
        lower: &Self::BigUint,
        greater: &Self::BigUint,
    ) -> SCResult<()> {
        require!(lower < greater, "K invariant failed");
        Ok(())
    }

    #[view]
    fn getTokensForGivenPosition(
        &self,
        liquidity: Self::BigUint,
    ) -> MultiResult2<FftTokenAmountPair<Self::BigUint>, FftTokenAmountPair<Self::BigUint>> {
        self.get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(
        &self,
    ) -> MultiResult3<Self::BigUint, Self::BigUint, Self::BigUint> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let total_supply = self.total_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view]
    fn getAmountOut(
        &self,
        token_in: TokenIdentifier,
        amount_in: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
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

    #[view]
    fn getAmountIn(
        &self,
        token_wanted: TokenIdentifier,
        amount_wanted: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
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

    #[view]
    fn getEquivalent(
        &self,
        token_in: TokenIdentifier,
        amount_in: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(amount_in > 0, "Zero input");
        let zero = Self::BigUint::zero();

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

    #[view]
    fn getLpTokenIdentifier(&self) -> TokenIdentifier {
        self.lp_token_identifier().get()
    }

    #[view(getTemporaryFunds)]
    #[storage_mapper("funds")]
    fn temporary_funds(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[storage_mapper("lpTokenIdentifier")]
    fn lp_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
