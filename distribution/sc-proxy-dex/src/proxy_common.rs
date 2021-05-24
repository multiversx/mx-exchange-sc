elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait ProxyCommonModule {
    #[endpoint(addAcceptedLockedAssetTokenId)]
    fn add_accepted_locked_asset_token_id(&self, token_id: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        self.accepted_locked_assets().insert(token_id);
        Ok(())
    }

    #[endpoint(removeAcceptedLockedAssetTokenId)]
    fn remove_accepted_locked_asset_token_id(&self, token_id: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        self.require_is_accepted_locked_asset(&token_id)?;
        self.accepted_locked_assets().remove(&token_id);
        Ok(())
    }

    fn require_is_accepted_locked_asset(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        require!(
            self.accepted_locked_assets().contains(token_id),
            "Not an accepted locked asset"
        );
        Ok(())
    }

    fn require_permissions(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    #[view(getAcceptedLockedAssetsTokenIds)]
    #[storage_mapper("accepted_locked_assets")]
    fn accepted_locked_assets(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("distributed_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
