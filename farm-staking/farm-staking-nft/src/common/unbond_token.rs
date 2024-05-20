multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnbondTokenModule: permissions_module::PermissionsModule {
    #[payable("EGLD")]
    #[endpoint(registerUnbondToken)]
    fn register_unbond_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) {
        self.require_caller_has_owner_or_admin_permissions();

        let payment_amount = self.call_value().egld_value().clone_value();
        self.unbond_token().issue_and_set_all_roles(
            EsdtTokenType::NonFungible,
            payment_amount,
            token_display_name,
            token_ticker,
            0,
            None,
        );
    }

    #[view(getUnbondTokenId)]
    #[storage_mapper("unbond_token_id")]
    fn unbond_token(&self) -> NonFungibleTokenMapper;
}
