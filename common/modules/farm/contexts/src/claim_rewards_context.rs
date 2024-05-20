multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::{ERROR_BAD_PAYMENTS, ERROR_EMPTY_PAYMENTS};
use common_structs::PaymentAttributesPair;
use multiversx_sc::api::BlockchainApi;
use multiversx_sc::contract_base::BlockchainWrapper;

pub struct ClaimRewardsContext<M, T>
where
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub first_farm_token: PaymentAttributesPair<M, T>,
    pub additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

impl<M, T> ClaimRewardsContext<M, T>
where
    M: ManagedTypeApi + BlockchainApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub fn new(
        mut payments: ManagedVec<M, EsdtTokenPayment<M>>,
        farm_token_id: &TokenIdentifier<M>,
        api_wrapper: BlockchainWrapper<M>,
    ) -> Self {
        if payments.is_empty() {
            M::error_api_impl().signal_error(ERROR_EMPTY_PAYMENTS);
        }

        for p in &payments {
            if &p.token_identifier != farm_token_id {
                M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
            }
        }

        let first_payment = payments.get(0);
        payments.remove(0);

        let own_sc_address = api_wrapper.get_sc_address();
        let token_data = api_wrapper.get_esdt_token_data(
            &own_sc_address,
            farm_token_id,
            first_payment.token_nonce,
        );
        let first_token_attributes: T = token_data.decode_attributes();

        ClaimRewardsContext {
            first_farm_token: PaymentAttributesPair {
                payment: first_payment,
                attributes: first_token_attributes,
            },
            additional_payments: payments,
        }
    }
}

pub type CompoundRewardsContext<M, T> = ClaimRewardsContext<M, T>;
