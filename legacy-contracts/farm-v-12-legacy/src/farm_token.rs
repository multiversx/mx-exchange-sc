multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{FarmTokenAttributes, Nonce};

use super::config;

static SET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"setSpecialRole";
static TRANSFER_ROLE_NAME: &[u8] = b"ESDTTransferRole";

#[derive(ManagedVecItem, Clone)]
pub struct FarmToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: FarmTokenAttributes<M>,
}

#[multiversx_sc::module]
pub trait FarmTokenModule: config::ConfigModule + token_send::TokenSendModule {
    #[only_owner]
    #[endpoint(setTransferRoleFarmToken)]
    fn set_transfer_role_farm_token(
        &self,
        #[var_args] opt_address: OptionalArg<ManagedAddress>,
    ) -> AsyncCall {
        let farm_token_id = self.farm_token_id().get();
        self.role_management_common(farm_token_id, SET_SPECIAL_ROLE_ENDPOINT_NAME, opt_address)
    }

    fn role_management_common(
        &self,
        locked_token_id: TokenIdentifier,
        endpoint_name: &[u8],
        opt_address: OptionalArg<ManagedAddress>,
    ) -> AsyncCall {
        let role_dest_address = self.resolve_address(opt_address);
        let esdt_system_sc_addr = self.send().esdt_system_sc_proxy().esdt_system_sc_address();
        let mut contract_call = ContractCall::<Self::Api, ()>::new(
            self.raw_vm_api(),
            esdt_system_sc_addr,
            ManagedBuffer::new_from_bytes(endpoint_name),
        );
        contract_call.push_endpoint_arg(&locked_token_id);
        contract_call.push_endpoint_arg(&role_dest_address);
        contract_call.push_endpoint_arg(&TRANSFER_ROLE_NAME);

        contract_call.async_call()
    }

    fn resolve_address(&self, opt_address: OptionalArg<ManagedAddress>) -> ManagedAddress {
        match opt_address {
            OptionalArg::Some(addr) => addr,
            OptionalArg::None => self.blockchain().get_sc_address(),
        }
    }

    fn decode_attributes(&self, attributes_raw: &ManagedBuffer) -> FarmTokenAttributes<Self::Api> {
        self.serializer()
            .top_decode_from_managed_buffer::<FarmTokenAttributes<Self::Api>>(attributes_raw)
    }

    fn get_farm_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> FarmTokenAttributes<Self::Api> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        self.serializer()
            .top_decode_from_managed_buffer::<FarmTokenAttributes<Self::Api>>(
                &token_info.attributes,
            )
    }

    fn burn_farm_tokens_from_payments(
        &self,
        payments: ManagedVecIterator<EsdtTokenPayment<Self::Api>>,
    ) {
        let mut total_amount = BigUint::zero();
        for entry in payments {
            total_amount += &entry.amount;
            self.send()
                .esdt_local_burn(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
        self.farm_token_supply().update(|x| *x -= total_amount);
    }

    fn burn_farm_tokens(&self, token_id: &TokenIdentifier, nonce: Nonce, amount: &BigUint) {
        self.send().esdt_local_burn(token_id, nonce, amount);
        self.farm_token_supply().update(|x| *x -= amount);
    }
}
