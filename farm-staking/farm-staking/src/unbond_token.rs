multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnbondTokenModule:
    permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("EGLD")]
    #[endpoint(registerUnbondToken)]
    fn register_unbond_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        self.require_caller_has_owner_or_admin_permissions();

        let payment_amount = self.call_value().egld_value().clone_value();
        self.unbond_token().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[view(getUnbondTokenId)]
    #[storage_mapper("unbondTokenId")]
    fn unbond_token(&self) -> NonFungibleTokenMapper;
}
