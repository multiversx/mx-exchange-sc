#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_EXTERN_SWAP_GAS_LIMIT: u64 = 50000000;

mod amm;
pub mod bot_protection;
pub mod config;
mod contexts;
pub mod errors;
mod events;
pub mod fee;
mod liquidity_pool;
pub mod locking_wrapper;
pub mod safe_price;

use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::errors::*;

use contexts::base::*;
use contexts::swap::SwapContext;
use pausable::State;

pub type AddLiquidityResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

pub type SwapTokensFixedOutputResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Pair<ContractReader>:
    amm::AmmModule
    + fee::FeeModule
    + liquidity_pool::LiquidityPoolModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + events::EventsModule
    + safe_price::SafePriceModule
    + bot_protection::BPModule
    + contexts::output_builder::OutputBuilderModule
    + locking_wrapper::LockingWrapperModule
    + pausable::PausableModule
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
        initial_liquidity_adder: OptionalValue<ManagedAddress>,
    ) {
        require!(first_token_id.is_valid_esdt_identifier(), ERROR_NOT_AN_ESDT);
        require!(
            second_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(first_token_id != second_token_id, ERROR_SAME_TOKENS);
        let lp_token_id = self.lp_token_identifier().get();
        require!(first_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        require!(second_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        self.set_fee_percents(total_fee_percent, special_fee_percent);

        self.state().set(State::Inactive);
        self.extern_swap_gas_limit()
            .set_if_empty(DEFAULT_EXTERN_SWAP_GAS_LIMIT);

        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.first_token_id().set(&first_token_id);
        self.second_token_id().set(&second_token_id);
        self.initial_liquidity_adder()
            .set(&initial_liquidity_adder.into_option());

        let pause_whitelist = self.pause_whitelist();
        pause_whitelist.add(&router_address);
        pause_whitelist.add(&router_owner_address);
    }

    #[payable("*")]
    #[endpoint(addInitialLiquidity)]
    fn add_initial_liquidity(&self) -> AddLiquidityResultType<Self::Api> {
        let mut storage_cache = StorageCache::new(self);
        let caller = self.blockchain().get_caller();

        let opt_initial_liq_adder = self.initial_liquidity_adder().get();
        if let Some(initial_liq_adder) = opt_initial_liq_adder {
            require!(caller == initial_liq_adder, ERROR_PERMISSION_DENIED);
        }

        let [first_payment, second_payment] = self.call_value().multi_esdt();
        require!(
            first_payment.token_identifier == storage_cache.first_token_id
                && first_payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );
        require!(
            second_payment.token_identifier == storage_cache.second_token_id
                && second_payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );
        require!(
            !self.is_state_active(storage_cache.contract_state),
            ERROR_ACTIVE
        );
        require!(
            storage_cache.lp_token_supply == 0,
            ERROR_INITIAL_LIQUIDITY_ALREADY_ADDED
        );

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let first_token_optimal_amount = &first_payment.amount;
        let second_token_optimal_amount = &second_payment.amount;
        let liq_added = self.pool_add_initial_liquidity(
            first_token_optimal_amount,
            second_token_optimal_amount,
            &mut storage_cache,
        );

        self.send()
            .esdt_local_mint(&storage_cache.lp_token_id, 0, &liq_added);
        self.send()
            .direct_esdt(&caller, &storage_cache.lp_token_id, 0, &liq_added);

        self.state().set(State::PartialActive);

        let add_liq_context = AddLiquidityContext {
            first_payment,
            second_payment,
            first_token_amount_min: BigUint::from(1u32),
            second_token_amount_min: BigUint::from(1u32),
            first_token_optimal_amount: first_token_optimal_amount.clone(),
            second_token_optimal_amount: second_token_optimal_amount.clone(),
            liq_added,
        };
        let output = self.build_add_initial_liq_results(&storage_cache, &add_liq_context);

        self.emit_add_liquidity_event(storage_cache, add_liq_context);

        output
    }

    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        require!(
            first_token_amount_min > 0 && second_token_amount_min > 0,
            ERROR_INVALID_ARGS
        );

        let mut storage_cache = StorageCache::new(self);
        let caller = self.blockchain().get_caller();

        let [first_payment, second_payment] = self.call_value().multi_esdt();
        require!(
            first_payment.token_identifier == storage_cache.first_token_id
                && first_payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );
        require!(
            second_payment.token_identifier == storage_cache.second_token_id
                && second_payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );
        require!(
            self.is_state_active(storage_cache.contract_state),
            ERROR_NOT_ACTIVE
        );
        require!(
            storage_cache.lp_token_id.is_valid_esdt_identifier(),
            ERROR_LP_TOKEN_NOT_ISSUED
        );
        require!(
            self.initial_liquidity_adder().get().is_none() || storage_cache.lp_token_supply != 0,
            ERROR_INITIAL_LIQUIDITY_NOT_ADDED
        );

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut add_liq_context = AddLiquidityContext::new(
            first_payment,
            second_payment,
            first_token_amount_min,
            second_token_amount_min,
        );
        self.set_optimal_amounts(&mut add_liq_context, &storage_cache);

        let liq_added = if storage_cache.lp_token_supply == 0u64 {
            self.pool_add_initial_liquidity(
                &add_liq_context.first_token_optimal_amount,
                &add_liq_context.second_token_optimal_amount,
                &mut storage_cache,
            )
        } else {
            self.pool_add_liquidity(
                &add_liq_context.first_token_optimal_amount,
                &add_liq_context.second_token_optimal_amount,
                &mut storage_cache,
            )
        };
        self.require_can_proceed_add(&storage_cache.lp_token_supply, &liq_added);
        add_liq_context.liq_added = liq_added;

        let new_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );
        require!(initial_k <= new_k, ERROR_K_INVARIANT_FAILED);

        self.send()
            .esdt_local_mint(&storage_cache.lp_token_id, 0, &add_liq_context.liq_added);

        let output_payments = self.build_add_liq_output_payments(&storage_cache, &add_liq_context);
        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        let output = self.build_add_liq_results(&storage_cache, &add_liq_context);

        self.emit_add_liquidity_event(storage_cache, add_liq_context);

        output
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiquidityResultType<Self::Api> {
        require!(
            first_token_amount_min > 0 && second_token_amount_min > 0,
            ERROR_INVALID_ARGS
        );

        let mut storage_cache = StorageCache::new(self);
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();

        require!(
            self.is_state_active(storage_cache.contract_state),
            ERROR_NOT_ACTIVE
        );
        require!(
            storage_cache.lp_token_id.is_valid_esdt_identifier(),
            ERROR_LP_TOKEN_NOT_ISSUED
        );
        require!(
            payment.token_identifier == storage_cache.lp_token_id && payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut remove_liq_context = RemoveLiquidityContext::new(
            payment.amount,
            first_token_amount_min,
            second_token_amount_min,
        );
        self.pool_remove_liquidity(&mut remove_liq_context, &mut storage_cache);
        self.require_can_proceed_remove(
            &storage_cache.lp_token_supply,
            &remove_liq_context.lp_token_payment_amount,
        );

        let new_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );
        require!(new_k <= initial_k, ERROR_K_INVARIANT_FAILED);

        self.burn(
            &storage_cache.lp_token_id,
            &remove_liq_context.lp_token_payment_amount,
        );

        let output_payments =
            self.build_remove_liq_output_payments(storage_cache, remove_liq_context);
        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_remove_liquidity_event(storage_cache, remove_liq_context);

        self.build_remove_liq_results(output_payments)
    }

    #[payable("*")]
    #[endpoint(removeLiquidityAndBuyBackAndBurnToken)]
    fn remove_liquidity_and_burn_token(&self, token_to_buyback_and_burn: TokenIdentifier) {
        let mut storage_cache = StorageCache::new(self);
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();

        require!(self.whitelist().contains(&caller), ERROR_NOT_WHITELISTED);
        require!(
            storage_cache.lp_token_id.is_valid_esdt_identifier(),
            ERROR_LP_TOKEN_NOT_ISSUED
        );
        require!(
            payment.token_identifier == storage_cache.lp_token_id && payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut remove_liq_context =
            RemoveLiquidityContext::new(payment.amount, BigUint::from(1u64), BigUint::from(1u64));
        self.pool_remove_liquidity(&mut remove_liq_context, &mut storage_cache);
        self.require_can_proceed_remove(
            &storage_cache.lp_token_supply,
            &remove_liq_context.lp_token_payment_amount,
        );

        self.burn(
            &storage_cache.lp_token_id,
            &remove_liq_context.lp_token_payment_amount,
        );

        let dest_address = ManagedAddress::zero();
        let first_token_id = storage_cache.first_token_id.clone();
        self.send_fee_slice(
            &mut storage_cache,
            SwapTokensOrder::PoolOrder,
            &first_token_id,
            &remove_liq_context.first_token_amount_removed,
            &dest_address,
            &token_to_buyback_and_burn,
        );

        let second_token_id = storage_cache.second_token_id.clone();
        self.send_fee_slice(
            &mut storage_cache,
            SwapTokensOrder::ReverseOrder,
            &second_token_id,
            &remove_liq_context.second_token_amount_removed,
            &dest_address,
            &token_to_buyback_and_burn,
        );
    }

    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(&self, token_out: TokenIdentifier, destination_address: ManagedAddress) {
        let (token_in, nonce, amount_in) = self.call_value().single_esdt().into_tuple();
        let mut context = self.new_swap_context(
            &token_in,
            nonce,
            &amount_in,
            token_out.clone(),
            BigUint::from(1u64),
        );
        require!(
            self.whitelist().contains(context.get_caller()),
            ERROR_NOT_WHITELISTED
        );
        require!(
            context.get_tx_input().get_args().are_valid(),
            ERROR_INVALID_ARGS
        );
        require!(
            context.get_tx_input().get_payments().are_valid(),
            ERROR_INVALID_PAYMENTS
        );
        require!(
            context.get_tx_input().is_valid(),
            ERROR_ARGS_NOT_MATCH_PAYMENTS
        );

        self.load_state(&mut context);
        require!(
            self.can_swap(context.get_contract_state()),
            ERROR_SWAP_NOT_ENABLED
        );

        self.load_pool_token_ids(&mut context);
        require!(context.input_tokens_match_pool_tokens(), ERROR_INVALID_ARGS);

        self.load_pool_reserves(&mut context);
        self.update_safe_state_from_context(&context);
        self.load_initial_k(&mut context);

        context.set_final_input_amount(amount_in.clone());
        let amount_out = self.swap_safe_no_fee(&mut context, &token_in, &amount_in);
        require!(amount_out > 0u64, ERROR_ZERO_AMOUNT);
        context.set_final_output_amount(amount_out.clone());

        self.require_can_proceed_swap(&context);

        let new_k = self.calculate_k(&context);
        require!(context.get_initial_k() <= &new_k, ERROR_K_INVARIANT_FAILED);

        self.commit_changes(&context);
        self.burn(&token_out, &amount_out);
        self.emit_swap_no_fee_and_forward_event(&context, &destination_address);
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        token_out: TokenIdentifier,
        amount_out_min: BigUint,
    ) -> SwapTokensFixedInputResultType<Self::Api> {
        require!(amount_out_min > 0, ERROR_INVALID_ARGS);

        let mut storage_cache = StorageCache::new(self);
        let (token_in, _, amount_in) = self.call_value().single_esdt().into_tuple();
        let swap_tokens_order = storage_cache.get_swap_tokens_order(&token_in, &token_out);

        require!(
            self.can_swap(storage_cache.contract_state),
            ERROR_SWAP_NOT_ENABLED
        );

        let reserve_out = storage_cache.get_reserve_out(swap_tokens_order);
        require!(*reserve_out > amount_out_min, ERROR_NOT_ENOUGH_RESERVE);

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let swap_context = SwapContext::new(
            token_in,
            amount_in,
            token_out,
            amount_out_min,
            swap_tokens_order,
        );
        self.perform_swap_fixed_input(&mut swap_context, &mut storage_cache);
        self.require_can_proceed_swap(&swap_context, &storage_cache);

        let new_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );
        require!(initial_k <= new_k, ERROR_K_INVARIANT_FAILED);

        if swap_context.fee_amount > 0 {
            self.send_fee(
                &mut storage_cache,
                swap_context.swap_tokens_order,
                &swap_context.input_token_id,
                &swap_context.fee_amount,
            );
        }

        let caller = self.blockchain().get_caller();
        let output_payments = self.build_swap_output_payments(&swap_context);
        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_swap_event(storage_cache, swap_context);

        self.build_swap_fixed_input_results(output_payments)
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        token_out: TokenIdentifier,
        amount_out: BigUint,
    ) -> SwapTokensFixedOutputResultType<Self::Api> {
        require!(amount_out > 0, ERROR_INVALID_ARGS);

        let mut storage_cache = StorageCache::new(self);
        let (token_in, _, amount_in_max) = self.call_value().single_esdt().into_tuple();
        let swap_tokens_order = storage_cache.get_swap_tokens_order(&token_in, &token_out);

        require!(
            self.can_swap(storage_cache.contract_state),
            ERROR_SWAP_NOT_ENABLED
        );

        let reserve_out = storage_cache.get_reserve_out(swap_tokens_order);
        require!(*reserve_out > amount_out, ERROR_NOT_ENOUGH_RESERVE);

        self.update_safe_state(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let swap_context = SwapContext::new(
            token_in,
            amount_in_max,
            token_out,
            amount_out,
            swap_tokens_order,
        );
        self.perform_swap_fixed_output(&mut swap_context, &mut storage_cache);
        self.require_can_proceed_swap(&swap_context, &storage_cache);

        let new_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );
        require!(initial_k <= new_k, ERROR_K_INVARIANT_FAILED);

        if swap_context.fee_amount > 0 {
            self.send_fee(
                &mut storage_cache,
                swap_context.swap_tokens_order,
                &swap_context.input_token_id,
                &swap_context.fee_amount,
            );
        }

        let caller = self.blockchain().get_caller();
        let output_payments = self.build_swap_output_payments(&swap_context);
        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_swap_event(storage_cache, swap_context);

        self.build_swap_fixed_output_results(output_payments)
    }

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) {
        self.require_permissions();
        require!(
            self.lp_token_identifier().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED
        );
        require!(
            token_identifier != self.first_token_id().get()
                && token_identifier != self.second_token_id().get(),
            ERROR_LP_TOKEN_SAME_AS_POOL_TOKENS
        );
        require!(
            token_identifier.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        self.lp_token_identifier().set(&token_identifier);
    }

    #[view(getTokensForGivenPosition)]
    fn get_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        self.get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(&self) -> MultiValue3<BigUint, BigUint, BigUint> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let total_supply = self.lp_token_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out_view(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        require!(amount_in > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_in == first_token_id {
            require!(second_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &first_token_reserve, &second_token_reserve);
            require!(second_token_reserve > amount_out, ERROR_NOT_ENOUGH_RESERVE);
            amount_out
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &second_token_reserve, &first_token_reserve);
            require!(first_token_reserve > amount_out, ERROR_NOT_ENOUGH_RESERVE);
            amount_out
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in_view(&self, token_wanted: TokenIdentifier, amount_wanted: BigUint) -> BigUint {
        require!(amount_wanted > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            require!(
                first_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE
            );

            self.get_amount_in(&amount_wanted, &second_token_reserve, &first_token_reserve)
        } else if token_wanted == second_token_id {
            require!(
                second_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE
            );

            self.get_amount_in(&amount_wanted, &first_token_reserve, &second_token_reserve)
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getEquivalent)]
    fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        require!(amount_in > 0u64, ERROR_ZERO_AMOUNT);
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
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }

    #[inline]
    fn is_state_active(&self, state: State) -> bool {
        state == State::Active || state == State::PartialActive
    }

    #[inline]
    fn can_swap(&self, state: State) -> bool {
        state == State::Active
    }

    fn perform_swap_fixed_input(
        &self,
        context: &mut SwapContext<Self::Api>,
        storage_cache: &mut StorageCache<Self>,
    ) {
        context.final_input_amount = context.input_token_amount.clone();

        let reserve_in = storage_cache.get_reserve_in(context.swap_tokens_order);
        let reserve_out = storage_cache.get_reserve_out(context.swap_tokens_order);

        let amount_out_optimal =
            self.get_amount_out(&context.input_token_amount, reserve_in, reserve_out);
        require!(
            amount_out_optimal >= context.output_token_amount,
            ERROR_SLIPPAGE_EXCEEDED
        );
        require!(*reserve_out > amount_out_optimal, ERROR_NOT_ENOUGH_RESERVE);
        require!(amount_out_optimal != 0u64, ERROR_ZERO_AMOUNT);

        context.final_output_amount = amount_out_optimal;

        let mut amount_in_after_fee = context.input_token_amount.clone();
        if self.is_fee_enabled() {
            let fee_amount = self.get_special_fee_from_input(&amount_in_after_fee);
            amount_in_after_fee -= &fee_amount;

            context.fee_amount = fee_amount;
        }

        *reserve_in += amount_in_after_fee;
        *reserve_out -= &context.final_output_amount;
    }

    fn perform_swap_fixed_output(
        &self,
        context: &mut SwapContext<Self::Api>,
        storage_cache: &mut StorageCache<Self>,
    ) {
        context.final_output_amount = context.output_token_amount.clone();

        let reserve_in = storage_cache.get_reserve_in(context.swap_tokens_order);
        let reserve_out = storage_cache.get_reserve_out(context.swap_tokens_order);

        let amount_in_optimal =
            self.get_amount_in(&context.output_token_amount, reserve_in, reserve_out);
        require!(
            amount_in_optimal <= context.input_token_amount,
            ERROR_SLIPPAGE_EXCEEDED
        );
        require!(amount_in_optimal != 0, ERROR_ZERO_AMOUNT);

        context.final_input_amount = amount_in_optimal.clone();

        let mut amount_in_optimal_after_fee = amount_in_optimal;
        if self.is_fee_enabled() {
            let fee_amount = self.get_special_fee_from_input(&amount_in_optimal_after_fee);
            amount_in_optimal_after_fee -= &fee_amount;

            context.fee_amount = fee_amount;
        }

        *reserve_in += amount_in_optimal_after_fee;
        *reserve_out -= &context.final_output_amount;
    }
}
