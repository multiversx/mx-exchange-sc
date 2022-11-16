elrond_wasm::imports!();

static SET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"setSpecialRole";
static UNSET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"unSetSpecialRole";
static TRANSFER_ROLE_NAME: &[u8] = b"ESDTTransferRole";

#[elrond_wasm::module]
pub trait TransferRoleModule:
    crate::locked_asset::LockedAssetModule
    + token_send::TokenSendModule
    + crate::attr_ex_helper::AttrExHelper
{
    /// Sets the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedToken)]
    fn set_transfer_role_locked_token(
        &self,
        #[var_args] opt_address: OptionalValue<ManagedAddress>,
    ) {
        self.role_management_common(SET_SPECIAL_ROLE_ENDPOINT_NAME, opt_address);
    }

    /// Removes the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(unsetTransferRoleLockedToken)]
    fn unset_transfer_role_locked_token(
        &self,
        #[var_args] opt_address: OptionalValue<ManagedAddress>,
    ) {
        self.role_management_common(UNSET_SPECIAL_ROLE_ENDPOINT_NAME, opt_address);
    }

    fn role_management_common(
        &self,
        endpoint_name: &[u8],
        opt_address: OptionalValue<ManagedAddress>,
    ) -> ! {
        let locked_token_id = self.locked_asset_token_id().get();
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
