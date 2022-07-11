elrond_wasm::imports!();

use crate::{AddLiquidityResultType, RemoveLiquidityResultType};

use super::{
    add_liquidity::AddLiquidityContext, base::StorageCache,
    remove_liquidity::RemoveLiquidityContext,
};

#[elrond_wasm::module]
pub trait OutputBuilderModule {
    fn build_add_initial_liq_results(
        &self,
        storage_cache: StorageCache<Self::Api>,
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
        storage_cache: &StorageCache<Self::Api>,
        add_liq_context: &AddLiquidityContext<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            storage_cache.lp_token_id,
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
        storage_cache: &StorageCache<Self::Api>,
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
        storage_cache: StorageCache<Self::Api>,
        remove_liq_context: RemoveLiquidityContext<Self::Api>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            storage_cache.first_token_id,
            0,
            remove_liq_context.first_token_amount_removed,
        ));
        payments.push(EsdtTokenPayment::new(
            storage_cache.second_token_id,
            0,
            remove_liq_context.second_token_amount_removed,
        ));

        payments
    }

    fn build_remove_liq_results(
        &self,
        output_payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> RemoveLiquidityResultType<Self::Api> {
        (output_payments.get(0), output_payments.get(1)).into()
    }
}
