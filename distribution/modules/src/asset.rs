elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINT_TOKENS_GAS_LIMIT: u64 = 5000000;

#[elrond_wasm_derive::module]
pub trait AssetModule {
    fn mint_and_send_assets(&self, address: &Address, amount: &Self::BigUint) {
        if amount > &0 {
            let token_id = self.asset_token_id().get();
            self.send().esdt_local_mint(
                MINT_TOKENS_GAS_LIMIT,
                &token_id.as_esdt_identifier(),
                amount,
            );
            self.send().transfer_tokens(&token_id, 0, amount, address);
        }
    }

    #[storage_mapper("distributed_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
