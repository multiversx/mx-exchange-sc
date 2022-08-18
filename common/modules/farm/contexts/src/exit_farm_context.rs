elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::ERROR_BAD_PAYMENTS;
use common_structs::FarmTokenAttributes;
use elrond_wasm::contract_base::BlockchainWrapper;
use farm_token::FarmToken;

pub struct ExitFarmContext<M: ManagedTypeApi> {
    pub farm_token: FarmToken<M>,
}

impl<M: ManagedTypeApi + BlockchainApi> ExitFarmContext<M> {
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
        let attributes: FarmTokenAttributes<M> = token_data.decode_attributes();

        ExitFarmContext {
            farm_token: FarmToken {
                payment,
                attributes,
            },
        }
    }
}
