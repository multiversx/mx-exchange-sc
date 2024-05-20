multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::{ERROR_BAD_PAYMENTS, ERROR_EMPTY_PAYMENTS};
use common_structs::PaymentsVec;
use multiversx_sc::{api::BlockchainApi, contract_base::BlockchainWrapper};

pub struct EnterFarmContext<
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
> {
    pub farming_token_payment: EsdtTokenPayment<M>,
    pub additional_farm_tokens: PaymentsVec<M>,
    pub additional_attributes: ManagedVec<M, T>,
}

impl<
        M: ManagedTypeApi + BlockchainApi,
        T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
    > EnterFarmContext<M, T>
{
    pub fn new(
        mut payments: PaymentsVec<M>,
        farming_token_id: &TokenIdentifier<M>,
        farm_token_id: &TokenIdentifier<M>,
        api_wrapper: BlockchainWrapper<M>,
    ) -> Self {
        if payments.is_empty() {
            M::error_api_impl().signal_error(ERROR_EMPTY_PAYMENTS);
        }

        let farming_token_payment = payments.get(0);
        if &farming_token_payment.token_identifier != farming_token_id {
            M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
        }

        payments.remove(0);

        let own_sc_address = api_wrapper.get_sc_address();
        let mut additional_attributes = ManagedVec::new();
        for p in &payments {
            if &p.token_identifier != farm_token_id {
                M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
            }

            let token_data =
                api_wrapper.get_esdt_token_data(&own_sc_address, farm_token_id, p.token_nonce);
            let token_attributes: T = token_data.decode_attributes();
            additional_attributes.push(token_attributes);
        }

        EnterFarmContext {
            farming_token_payment,
            additional_farm_tokens: payments,
            additional_attributes,
        }
    }
}
