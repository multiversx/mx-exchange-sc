elrond_wasm::imports!();

static SET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"setSpecialRole";
static UNSET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"unSetSpecialRole";
static TRANSFER_ROLE_NAME: &[u8] = b"ESDTTransferRole";

use super::proxy_common;

#[elrond_wasm::module]
pub trait TransferRoleModule: proxy_common::ProxyCommonModule {
    /// Sets the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedLpToken)]
    fn set_transfer_role_locked_lp_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_lp_token_id = self.wrapped_lp_token_id().get();
        self.role_management_common(
            locked_lp_token_id,
            SET_SPECIAL_ROLE_ENDPOINT_NAME,
            opt_address,
        );
    }

    /// Removes the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(unsetTransferRoleLockedLpToken)]
    fn unset_transfer_role_locked_lp_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_lp_token_id = self.wrapped_lp_token_id().get();
        self.role_management_common(
            locked_lp_token_id,
            UNSET_SPECIAL_ROLE_ENDPOINT_NAME,
            opt_address,
        );
    }

    /// Sets the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedFarmToken)]
    fn set_transfer_role_locked_farm_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_farm_token_id = self.wrapped_farm_token_id().get();
        self.role_management_common(
            locked_farm_token_id,
            SET_SPECIAL_ROLE_ENDPOINT_NAME,
            opt_address,
        );
    }

    /// Removes the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(unsetTransferRoleLockedFarmToken)]
    fn unset_transfer_role_locked_farm_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_farm_token_id = self.wrapped_farm_token_id().get();
        self.role_management_common(
            locked_farm_token_id,
            UNSET_SPECIAL_ROLE_ENDPOINT_NAME,
            opt_address,
        );
    }

    fn role_management_common(
        &self,
        locked_token_id: TokenIdentifier,
        endpoint_name: &[u8],
        opt_address: OptionalValue<ManagedAddress>,
    ) -> ! {
        let role_dest_address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        let esdt_system_sc_addr = self.send().esdt_system_sc_proxy().esdt_system_sc_address();
        let mut contract_call = ContractCall::<_, ()>::new(
            esdt_system_sc_addr,
            ManagedBuffer::new_from_bytes(endpoint_name),
        );
        contract_call.push_endpoint_arg(&locked_token_id);
        contract_call.push_endpoint_arg(&role_dest_address);
        contract_call.push_endpoint_arg(&TRANSFER_ROLE_NAME);

        contract_call.async_call().call_and_exit();
    }
}
