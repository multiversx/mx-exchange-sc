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
        kill!(self, first_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        kill!(self, second_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        kill!(self, first_token_id != second_token_id, ERROR_SAME_TOKENS);
        let lp_token_id = self.lp_token_identifier().get();
        kill!(self, first_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        kill!(
            self,
            second_token_id != lp_token_id,
            ERROR_POOL_TOKEN_IS_PLT
        );
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
        kill!(
            self,
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        kill!(
            self,
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        kill!(
            self,
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_state(&mut context);
        kill!(
            self,
            self.is_state_active(context.get_contract_state()),
            ERROR_NOT_ACTIVE,
        );

        self.load_lp_token_id(&mut context);
        kill!(
            self,
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );

        self.load_pool_token_ids(&mut context);
        kill!(
            self,
            context.payment_tokens_match_pool_tokens(),
            ERROR_BAD_PAYMENT_TOKENS,
        );

        self.load_pool_reserves(&mut context);
        self.load_lp_token_supply(&mut context);
        self.load_initial_k(&mut context);

        self.calculate_optimal_amounts(&mut context);
        self.pool_add_liquidity(&mut context);

        let new_k = self.calculate_k(&context);
        kill!(
            self,
            context.get_initial_k() <= &new_k,
            ERROR_K_INVARIANT_FAILED
        );

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
        kill!(
            self,
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        kill!(
            self,
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        kill!(
            self,
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_state(&mut context);
        kill!(
            self,
            self.is_state_active(context.get_contract_state()),
            ERROR_NOT_ACTIVE,
        );

        self.load_lp_token_id(&mut context);
        kill!(
            self,
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );
        kill!(
            self,
            context.get_lp_token_id() == &context.get_lp_token_payment().token_identifier,
            ERROR_BAD_PAYMENT_TOKENS,
        );

        self.load_pool_token_ids(&mut context);
        self.load_pool_reserves(&mut context);
        self.load_lp_token_supply(&mut context);
        self.load_initial_k(&mut context);

        self.pool_remove_liquidity(&mut context);

        let new_k = self.calculate_k(&context);
        kill!(
            self,
            &new_k <= context.get_initial_k(),
            ERROR_K_INVARIANT_FAILED
        );

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
        kill!(
            self,
            self.whitelist().contains(context.get_caller()),
            ERROR_NOT_WHITELISTED,
        );

        kill!(
            self,
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS,
        );
        kill!(
            self,
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS,
        );
        kill!(
            self,
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS,
        );

        self.load_lp_token_id(&mut context);
        kill!(
            self,
            !context.get_lp_token_id().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );
        kill!(
            self,
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
    ) {
        let caller = self.blockchain().get_caller();
        kill!(
            self,
            self.whitelist().contains(&caller),
            ERROR_NOT_WHITELISTED
        );
        kill!(self, self.can_swap(), ERROR_SWAP_NOT_ENABLED);
        kill!(self, amount_in > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        kill!(self, token_in != token_out, ERROR_SAME_TOKENS);
        kill!(
            self,
            token_in == first_token_id || token_in == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );
        kill!(
            self,
            token_out == first_token_id || token_out == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );

        let old_k = self.calculate_k_for_reserves();

        let amount_out =
            self.swap_safe_no_fee(&first_token_id, &second_token_id, &token_in, &amount_in);
        kill!(self, amount_out > 0u64, ERROR_ZERO_AMOUNT);

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        kill!(self, old_k <= new_k, ERROR_K_INVARIANT_FAILED);

        self.burn_fees(&token_out, &amount_out);

        self.emit_swap_no_fee_and_forward_event(
            &caller,
            &token_in,
            &amount_in,
            &token_out,
            &amount_out,
            &destination_address,
        );
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
    ) -> SwapTokensFixedInputResultType<Self::Api> {
        kill!(self, self.can_swap(), ERROR_SWAP_NOT_ENABLED);
        kill!(self, amount_in > 0u64, ERROR_ZERO_AMOUNT);
        kill!(self, token_in != token_out, ERROR_SAME_TOKENS);
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        kill!(
            self,
            token_in == first_token_id || token_in == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );
        kill!(
            self,
            token_out == first_token_id || token_out == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        kill!(
            self,
            reserve_token_out > amount_out_min,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
        let amount_out_optimal =
            self.get_amount_out(&amount_in, &reserve_token_in, &reserve_token_out);
        kill!(
            self,
            amount_out_optimal >= amount_out_min,
            ERROR_SLIPPAGE_EXCEEDED,
        );
        kill!(
            self,
            reserve_token_out > amount_out_optimal,
            ERROR_NOT_ENOUGH_RESERVE,
        );
        kill!(self, amount_out_optimal != 0u64, ERROR_ZERO_AMOUNT);

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
        kill!(self, old_k <= new_k, ERROR_K_INVARIANT_FAILED);

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
        )
        .unwrap();

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
        self.create_payment(&token_out, 0, &amount_out_optimal)
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
    ) -> SwapTokensFixedOutputResultType<Self::Api> {
        kill!(self, self.can_swap(), ERROR_SWAP_NOT_ENABLED);
        kill!(self, amount_in_max > 0u64, ERROR_ZERO_AMOUNT);
        kill!(self, token_in != token_out, ERROR_SAME_TOKENS);
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        kill!(
            self,
            token_in == first_token_id || token_in == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );
        kill!(
            self,
            token_out == first_token_id || token_out == second_token_id,
            ERROR_UNKNOWN_TOKEN,
        );
        kill!(self, amount_out != 0u64, ERROR_ZERO_AMOUNT);
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        kill!(
            self,
            reserve_token_out > amount_out,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
        let amount_in_optimal =
            self.get_amount_in(&amount_out, &reserve_token_in, &reserve_token_out);
        kill!(
            self,
            amount_in_optimal <= amount_in_max,
            ERROR_SLIPPAGE_EXCEEDED
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
        kill!(self, old_k <= new_k, ERROR_K_INVARIANT_FAILED);

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_fee_enabled() {
            self.send_fee(&token_in, &fee_amount);
        }

        let mut payments = ManagedVec::new();
        payments.push(self.create_payment(&token_out, 0, &amount_out));
        payments.push(self.create_payment(&token_in, 0, &residuum));
        self.send_multiple_tokens_if_not_zero(&caller, &payments, &opt_accept_funds_func)
            .unwrap();

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
        MultiResult2::from((
            self.create_payment(&token_out, 0, &amount_out),
            self.create_payment(&token_in, 0, &residuum),
        ))
    }

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) {
        self.require_permissions();
        kill!(
            self,
            self.lp_token_identifier().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED,
        );
        kill!(
            self,
            token_identifier != self.first_token_id().get()
                && token_identifier != self.second_token_id().get(),
            ERROR_LP_TOKEN_SAME_AS_POOL_TOKENS,
        );
        kill!(self, token_identifier.is_esdt(), ERROR_NOT_AN_ESDT);
        self.lp_token_identifier().set(&token_identifier);
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
    fn get_amount_out_view(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        kill!(self, amount_in > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_in == first_token_id {
            kill!(self, second_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &first_token_reserve, &second_token_reserve);
            kill!(
                self,
                second_token_reserve > amount_out,
                ERROR_NOT_ENOUGH_RESERVE
            );
            amount_out
        } else if token_in == second_token_id {
            kill!(self, first_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &second_token_reserve, &first_token_reserve);
            kill!(
                self,
                first_token_reserve > amount_out,
                ERROR_NOT_ENOUGH_RESERVE
            );
            amount_out
        } else {
            kill!(self, ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in_view(&self, token_wanted: TokenIdentifier, amount_wanted: BigUint) -> BigUint {
        kill!(self, amount_wanted > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            kill!(
                self,
                first_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE,
            );
            let amount_in =
                self.get_amount_in(&amount_wanted, &second_token_reserve, &first_token_reserve);
            amount_in
        } else if token_wanted == second_token_id {
            kill!(
                self,
                second_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE,
            );
            let amount_in =
                self.get_amount_in(&amount_wanted, &first_token_reserve, &second_token_reserve);
            amount_in
        } else {
            kill!(self, ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getEquivalent)]
    fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        kill!(self, amount_in > 0u64, ERROR_ZERO_AMOUNT);
        let zero = BigUint::zero();

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        if first_token_reserve == 0u64 || second_token_reserve == 0u64 {
            return zero;
        }

        if token_in == first_token_id {
            self.quote(&amount_in, &first_token_reserve, &second_token_reserve)
        } else if token_in == second_token_id {
            self.quote(&amount_in, &second_token_reserve, &first_token_reserve)
        } else {
            kill!(self, ERROR_UNKNOWN_TOKEN);
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
