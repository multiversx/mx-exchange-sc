elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait TokenWhitelistModule {
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
