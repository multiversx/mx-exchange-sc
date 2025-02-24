use crate::{
    contexts::remove_liquidity::RemoveLiquidityContext, StorageCache, SwapTokensOrder,
    ERROR_BAD_PAYMENT_TOKENS, ERROR_INVALID_ARGS, ERROR_K_INVARIANT_FAILED,
    ERROR_LP_TOKEN_NOT_ISSUED, ERROR_NOT_ACTIVE, ERROR_NOT_WHITELISTED, ERROR_SLIPPAGE_ON_REMOVE,
};

use super::common_result_types::RemoveLiquidityResultType;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RemoveLiquidityModule:
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
    + utils::UtilsModule
{
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

        self.update_safe_price(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
            &storage_cache.lp_token_supply,
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
            self.build_remove_liq_output_payments(&storage_cache, &remove_liq_context);
        let first_payment_after = output_payments.get(0);
        let second_payment_after = output_payments.get(1);
        require!(
            first_payment_after.amount >= remove_liq_context.first_token_amount_min,
            ERROR_SLIPPAGE_ON_REMOVE
        );
        require!(
            second_payment_after.amount >= remove_liq_context.second_token_amount_min,
            ERROR_SLIPPAGE_ON_REMOVE
        );

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_remove_liquidity_event(&storage_cache, remove_liq_context);

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

        self.update_safe_price(
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
            &storage_cache.lp_token_supply,
        );

        let mut remove_liq_context =
            RemoveLiquidityContext::new(payment.amount, BigUint::from(1u64), BigUint::from(1u64));
        self.pool_remove_liquidity(&mut remove_liq_context, &mut storage_cache);

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
}
