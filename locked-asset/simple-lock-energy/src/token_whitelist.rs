elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait TokenWhitelistModule:
    simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// Sets the LOCKED token ID. This is required, since we use an already existing token,
    /// instead of issue-ing a new one.
    ///
    /// The SC must already have the following roles before this function can be called:
    /// - NFTCReate
    /// - NFTAddQuantity
    /// - NFTBurn
    /// - TransferRole
    #[only_owner]
    #[endpoint(setLockedTokenId)]
    fn set_locked_token_id(&self, token_id: TokenIdentifier) {
        self.require_valid_token_id(&token_id);
        self.require_has_roles_for_locked_token(&token_id);

        self.locked_token().set_token_id(&token_id);
    }

    fn require_has_roles_for_locked_token(&self, token_id: &TokenIdentifier) {
        let actual_roles = self.blockchain().get_esdt_local_roles(token_id);
        let required_roles = EsdtLocalRoleFlags::NFT_CREATE
            | EsdtLocalRoleFlags::NFT_ADD_QUANTITY
            | EsdtLocalRoleFlags::NFT_BURN
            | EsdtLocalRoleFlags::TRANSFER;
        require!(
            actual_roles.contains(required_roles),
            "SC does not have ESDT transfer role for {}",
            token_id
        );
    }

    fn require_valid_token_id(&self, token_id: &TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), "Invalid token ID");
    }

    fn require_is_base_asset_token(&self, token_id: &TokenIdentifier) {
        let base_asset_id = self.base_asset_token_id().get();
        require!(
            token_id == &base_asset_id,
            "May only lock the whitelisted token"
        );
    }

    #[view(getBaseAssetTokenId)]
    #[storage_mapper("baseAssetTokenId")]
    fn base_asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
