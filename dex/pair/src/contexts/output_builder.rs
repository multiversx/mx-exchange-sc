elrond_wasm::imports!();

use crate::AddLiquidityResultType;

use super::{add_liquidity::AddLiquidityContext, base::StorageCache};

#[elrond_wasm::module]
pub trait OutputBuilderModule {
    fn build_add_initial_liq_results(
        &self,
        storage_cache: StorageCache<Self::Api>,
        liq_added: BigUint,
        first_token_optimal_amount: BigUint,
        second_token_optimal_amount: BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        (
            EsdtTokenPayment::new(storage_cache.lp_token_id, 0, liq_added),
            EsdtTokenPayment::new(storage_cache.first_token_id, 0, first_token_optimal_amount),
            EsdtTokenPayment::new(
                storage_cache.second_token_id,
                0,
                second_token_optimal_amount,
            ),
        )
            .into()
    }

    fn build_add_liq_output_payments(
        &self,
        storage_cache: &StorageCache<Self::Api>,
        add_liq_context: &AddLiquidityContext<Self::Api>,
        liq_added: BigUint,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            storage_cache.lp_token_id,
            0,
            liq_added,
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
        storage_cache: StorageCache<Self::Api>,
        add_liq_context: AddLiquidityContext<Self::Api>,
        liq_added: BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        (
            EsdtTokenPayment::new(storage_cache.lp_token_id, 0, liq_added),
            EsdtTokenPayment::new(
                storage_cache.first_token_id,
                0,
                add_liq_context.first_token_optimal_amount,
            ),
            EsdtTokenPayment::new(
                storage_cache.second_token_id,
                0,
                add_liq_context.second_token_optimal_amount,
            ),
        )
            .into()
    }
}
