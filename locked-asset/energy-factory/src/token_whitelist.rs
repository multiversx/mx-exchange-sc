multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait TokenWhitelistModule {
    fn is_base_asset_token(&self, token_id: &TokenIdentifier) -> bool {
        let base_asset_id = self.base_asset_token_id().get();
        token_id == &base_asset_id
    }

    #[view(getBaseAssetTokenId)]
    #[storage_mapper("baseAssetTokenId")]
    fn base_asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLegacyLockedTokenId)]
    #[storage_mapper("legacyLockedTokenId")]
    fn legacy_locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
