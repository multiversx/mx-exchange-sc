multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FarmTokenRolesModule:
    farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[endpoint(setBurnRoleForAddress)]
    fn set_burn_role_for_address(&self, address: ManagedAddress) {
        self.farm_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::NftBurn], None);
    }
}
