use crate::{
    contexts::add_liquidity::AddLiquidityContext, StorageCache, ERROR_BAD_PAYMENT_TOKENS,
    ERROR_INITIAL_LIQUIDITY_NOT_ADDED, ERROR_INVALID_ARGS, ERROR_K_INVARIANT_FAILED,
    ERROR_LP_TOKEN_NOT_ISSUED, ERROR_NOT_ACTIVE,
};

use super::common_result_types::AddLiquidityResultType;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait AddLiquidityModule:
    crate::liquidity_pool::LiquidityPoolModule
    + crate::amm::AmmModule
    + crate::contexts::output_builder::OutputBuilderModule
    + crate::locking_wrapper::LockingWrapperModule
    + crate::events::EventsModule
    + crate::safe_price::SafePriceModule
    + crate::config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + super::common_methods::CommonMethodsModule
{
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

        self.update_safe_price(
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

        add_liq_context.liq_added = if storage_cache.lp_token_supply == 0u64 {
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

        self.emit_add_liquidity_event(&storage_cache, add_liq_context);

        output
    }
}
