elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::ERROR_BAD_PAYMENTS;

pub struct ExitFarmContext<M: ManagedTypeApi> {
    pub farm_token_payment: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> ExitFarmContext<M> {
    pub fn new(payment: EsdtTokenPayment<M>, farm_token_id: &TokenIdentifier<M>) -> Self {
        if &payment.token_identifier != farm_token_id {
            M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
        }

        ExitFarmContext {
            farm_token_payment: payment,
        }
    }
}
