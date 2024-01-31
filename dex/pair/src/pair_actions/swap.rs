use crate::{
    contexts::swap::SwapContext, pair_hooks::hook_type::HookType, StorageCache, ERROR_INVALID_ARGS,
    ERROR_K_INVARIANT_FAILED, ERROR_NOT_ENOUGH_RESERVE, ERROR_NOT_WHITELISTED,
    ERROR_SLIPPAGE_EXCEEDED, ERROR_SWAP_NOT_ENABLED, ERROR_ZERO_AMOUNT,
};

use super::common_result_types::{SwapTokensFixedInputResultType, SwapTokensFixedOutputResultType};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum SwapType {
    FixedInput,
    FixedOutput,
}

#[multiversx_sc::module]
pub trait SwapModule:
    crate::liquidity_pool::LiquidityPoolModule
    + crate::amm::AmmModule
    + crate::contexts::output_builder::OutputBuilderModule
    + crate::locking_wrapper::LockingWrapperModule
    + crate::events::EventsModule
    + crate::safe_price::SafePriceModule
    + crate::fee::FeeModule
    + crate::config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + super::common_methods::CommonMethodsModule
    + crate::pair_hooks::banned_address::BannedAddressModule
    + crate::pair_hooks::change_hooks::ChangeHooksModule
    + crate::pair_hooks::call_hook::CallHookModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(&self, token_out: TokenIdentifier, destination_address: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        require!(self.whitelist().contains(&caller), ERROR_NOT_WHITELISTED);

        let mut storage_cache = StorageCache::new(self);
        let payment = self.call_value().single_esdt();
        let swap_tokens_order =
            storage_cache.get_swap_tokens_order(&payment.token_identifier, &token_out);

        require!(
            self.can_swap(storage_cache.contract_state),
            ERROR_SWAP_NOT_ENABLED
        );

        self.update_safe_price(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut swap_context = SwapContext::new(
            payment.token_identifier,
            payment.amount.clone(),
            token_out,
            BigUint::from(1u32),
            swap_tokens_order,
        );
        swap_context.final_input_amount = payment.amount;

        let amount_out = self.swap_safe_no_fee(
            &mut storage_cache,
            swap_context.swap_tokens_order,
            &swap_context.final_input_amount,
        );
        require!(amount_out > 0u64, ERROR_ZERO_AMOUNT);

        swap_context.final_output_amount = amount_out;

        let new_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );
        require!(initial_k <= new_k, ERROR_K_INVARIANT_FAILED);

        self.burn(
            &swap_context.output_token_id,
            &swap_context.final_output_amount,
        );

        self.emit_swap_no_fee_and_forward_event(swap_context, destination_address);
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
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let swap_tokens_order =
            storage_cache.get_swap_tokens_order(&payment.token_identifier, &token_out);

        require!(
            self.can_swap(storage_cache.contract_state),
            ERROR_SWAP_NOT_ENABLED
        );

        let reserve_out = storage_cache.get_mut_reserve_out(swap_tokens_order);
        require!(*reserve_out > amount_out_min, ERROR_NOT_ENOUGH_RESERVE);

        self.update_safe_price(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut args = ManagedVec::new();
        self.encode_arg_to_vec(&SwapType::FixedInput, &mut args);
        self.encode_arg_to_vec(&token_out, &mut args);
        self.encode_arg_to_vec(&amount_out_min, &mut args);

        let payments_after_hook = self.call_hook(
            HookType::BeforeSwap,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            args.clone(),
        );
        let payment = payments_after_hook.get(0);

        let mut swap_context = SwapContext::new(
            payment.token_identifier,
            payment.amount,
            token_out,
            amount_out_min,
            swap_tokens_order,
        );
        self.perform_swap_fixed_input(&mut swap_context, &mut storage_cache);

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
        let output_payments_after_hook =
            self.call_hook(HookType::AfterSwap, caller.clone(), output_payments, args);

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments_after_hook);

        self.emit_swap_event(&storage_cache, swap_context);

        self.build_swap_fixed_input_results(output_payments_after_hook)
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
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let swap_tokens_order =
            storage_cache.get_swap_tokens_order(&payment.token_identifier, &token_out);

        require!(
            self.can_swap(storage_cache.contract_state),
            ERROR_SWAP_NOT_ENABLED
        );

        let reserve_out = storage_cache.get_mut_reserve_out(swap_tokens_order);
        require!(*reserve_out > amount_out, ERROR_NOT_ENOUGH_RESERVE);

        self.update_safe_price(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let initial_k = self.calculate_k_constant(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        let mut args = ManagedVec::new();
        self.encode_arg_to_vec(&SwapType::FixedOutput, &mut args);
        self.encode_arg_to_vec(&token_out, &mut args);
        self.encode_arg_to_vec(&amount_out, &mut args);

        let payments_after_hook = self.call_hook(
            HookType::BeforeSwap,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            args.clone(),
        );
        let payment = payments_after_hook.get(0);

        let mut swap_context = SwapContext::new(
            payment.token_identifier,
            payment.amount,
            token_out,
            amount_out,
            swap_tokens_order,
        );
        self.perform_swap_fixed_output(&mut swap_context, &mut storage_cache);

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
        let output_payments_after_hook =
            self.call_hook(HookType::AfterSwap, caller.clone(), output_payments, args);

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments_after_hook);

        self.emit_swap_event(&storage_cache, swap_context);

        self.build_swap_fixed_output_results(output_payments_after_hook)
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

        *storage_cache.get_mut_reserve_in(context.swap_tokens_order) += amount_in_after_fee;
        *storage_cache.get_mut_reserve_out(context.swap_tokens_order) -=
            &context.final_output_amount;
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

        *storage_cache.get_mut_reserve_in(context.swap_tokens_order) += amount_in_optimal_after_fee;
        *storage_cache.get_mut_reserve_out(context.swap_tokens_order) -=
            &context.final_output_amount;
    }
}
