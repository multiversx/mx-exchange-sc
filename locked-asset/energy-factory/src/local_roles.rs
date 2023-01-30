multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait LocalRolesModule:
    simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// Sets the transfer role for the given address. Defaults to own address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedToken)]
    fn set_transfer_role(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.locked_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::Transfer], None);
    }

    /// Sets the burn role for the given address
    #[only_owner]
    #[endpoint(setBurnRoleLockedToken)]
    fn set_burn_role(&self, address: ManagedAddress) {
        self.locked_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::NftBurn], None);
    }
}
