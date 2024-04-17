multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::ERROR_BAD_PAYMENTS;
use common_structs::PaymentAttributesPair;
use multiversx_sc::api::BlockchainApi;
use multiversx_sc::contract_base::BlockchainWrapper;

pub struct ExitFarmContext<M, T>
where
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub farm_token: PaymentAttributesPair<M, T>,
}

impl<M, T> ExitFarmContext<M, T>
where
    M: ManagedTypeApi + BlockchainApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub fn new(
        payment: EsdtTokenPayment<M>,
        farm_token_id: &TokenIdentifier<M>,
        api_wrapper: BlockchainWrapper<M>,
    ) -> Self {
        if &payment.token_identifier != farm_token_id {
            M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
        }

        let own_sc_address = api_wrapper.get_sc_address();
        let token_data =
            api_wrapper.get_esdt_token_data(&own_sc_address, farm_token_id, payment.token_nonce);
        let attributes: T = token_data.decode_attributes();

        ExitFarmContext {
            farm_token: PaymentAttributesPair {
                payment,
                attributes,
            },
        }
    }
}
