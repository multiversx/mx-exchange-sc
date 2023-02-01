multiversx_sc::imports!();

use crate::{
    AddLiquidityResultType, RemoveLiquidityResultType, SwapTokensFixedInputResultType,
    SwapTokensFixedOutputResultType,
};

use super::{
    add_liquidity::AddLiquidityContext, base::StorageCache,
    remove_liquidity::RemoveLiquidityContext, swap::SwapContext,
};

#[multiversx_sc::module]
pub trait OutputBuilderModule:
    crate::config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + crate::locking_wrapper::LockingWrapperModule
{
    fn build_add_initial_liq_results(
        &self,
        storage_cache: &StorageCache<Self>,
        add_liq_context: &AddLiquidityContext<Self::Api>,
    ) -> AddLiquidityResultType<Self::Api> {
        (
            EsdtTokenPayment::new(
                storage_cache.lp_token_id.clone(),
                0,
                add_liq_context.liq_added.clone(),
            ),
            EsdtTokenPayment::new(
                storage_cache.first_token_id.clone(),
                0,
                add_liq_context.first_token_optimal_amount.clone(),
            ),
            EsdtTokenPayment::new(
                storage_cache.second_token_id.clone(),
                0,
                add_liq_context.second_token_optimal_amount.clone(),
            ),
        )
            .into()
    }

    fn build_add_liq_output_payments(
        &self,
        storage_cache: &StorageCache<Self>,
        add_liq_context: &AddLiquidityContext<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            storage_cache.lp_token_id.clone(),
            0,
            add_liq_context.liq_added.clone(),
        ));
        payments.push(EsdtTokenPayment::new(
            storage_cache.first_token_id.clone(),
            0,
            &add_liq_context.first_payment.amount - &add_liq_context.first_token_optimal_amount,
        ));
        payments.push(EsdtTokenPayment::new(
            storage_cache.second_token_id.clone(),
            0,
            &add_liq_context.second_payment.amount - &add_liq_context.second_token_optimal_amount,
        ));

        payments
    }

    fn build_add_liq_results(
        &self,
        storage_cache: &StorageCache<Self>,
        add_liq_context: &AddLiquidityContext<Self::Api>,
    ) -> AddLiquidityResultType<Self::Api> {
        (
            EsdtTokenPayment::new(
                storage_cache.lp_token_id.clone(),
                0,
                add_liq_context.liq_added.clone(),
            ),
            EsdtTokenPayment::new(
                storage_cache.first_token_id.clone(),
                0,
                add_liq_context.first_token_optimal_amount.clone(),
            ),
            EsdtTokenPayment::new(
                storage_cache.second_token_id.clone(),
                0,
                add_liq_context.second_token_optimal_amount.clone(),
            ),
        )
            .into()
    }

    fn build_remove_liq_output_payments(
        &self,
        storage_cache: &StorageCache<Self>,
        remove_liq_context: &RemoveLiquidityContext<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            storage_cache.first_token_id.clone(),
            0,
            remove_liq_context.first_token_amount_removed.clone(),
        ));
        payments.push(EsdtTokenPayment::new(
            storage_cache.second_token_id.clone(),
            0,
            remove_liq_context.second_token_amount_removed.clone(),
        ));

        payments
    }

    fn build_remove_liq_results(
        &self,
        output_payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> RemoveLiquidityResultType<Self::Api> {
        (output_payments.get(0), output_payments.get(1)).into()
    }

    fn build_swap_output_payments(
        &self,
        swap_context: &SwapContext<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments = ManagedVec::new();

        if self.should_generate_locked_asset() {
            let locked_asset = self.lock_tokens(
                swap_context.output_token_id.clone(),
                swap_context.final_output_amount.clone(),
            );
            payments.push(locked_asset);
        } else {
            payments.push(EsdtTokenPayment::new(
                swap_context.output_token_id.clone(),
                0,
                swap_context.final_output_amount.clone(),
            ));
        }

        if swap_context.final_input_amount < swap_context.input_token_amount {
            let extra_amount = &swap_context.input_token_amount - &swap_context.final_input_amount;
            payments.push(EsdtTokenPayment::new(
                swap_context.input_token_id.clone(),
                0,
                extra_amount,
            ));
        }

        payments
    }

    #[inline]
    fn build_swap_fixed_input_results(
        &self,
        output_payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SwapTokensFixedInputResultType<Self::Api> {
        output_payments.get(0)
    }

    fn build_swap_fixed_output_results(
        &self,
        output_payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SwapTokensFixedOutputResultType<Self::Api> {
        (output_payments.get(0), output_payments.get(1)).into()
    }
}
