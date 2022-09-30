elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait TokenWhitelistModule {
    fn is_base_asset_token(&self, token_id: &TokenIdentifier) -> bool {
        let base_asset_id = self.base_asset_token_id().get();
        token_id == &base_asset_id
    }

    fn is_legacy_locked_token(&self, token_id: &TokenIdentifier) -> bool {
        let legacy_locked_token_id = self.legacy_locked_token_id().get();
        token_id == &legacy_locked_token_id
    }

    #[view(getBaseAssetTokenId)]
    #[storage_mapper("baseAssetTokenId")]
    fn base_asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLegacyLockedTokenId)]
    #[storage_mapper("legacyLockedTokenId")]
    fn legacy_locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
