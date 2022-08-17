elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::ERROR_BAD_PAYMENTS;

pub struct ClaimRewardsContext<M: ManagedTypeApi> {
    pub farm_token_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

impl<M: ManagedTypeApi> ClaimRewardsContext<M> {
    pub fn new(
        payments: ManagedVec<M, EsdtTokenPayment<M>>,
        farm_token_id: &TokenIdentifier<M>,
    ) -> Self {
        for p in &payments {
            if &p.token_identifier != farm_token_id {
                M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
            }
        }

        ClaimRewardsContext {
            farm_token_payments: payments,
        }
    }
}

pub type CompoundRewardsContext<M> = ClaimRewardsContext<M>;
