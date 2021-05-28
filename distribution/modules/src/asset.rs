elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait AssetModule {
    fn mint_and_send_assets(&self, address: &Address, amount: &Self::BigUint) {
        if amount > &0 {
            let token_id = self.asset_token_id().get();
            self.send().esdt_local_mint(&token_id, amount);
            self.send().direct(address, &token_id, amount, &[]);
        }
    }

    #[storage_mapper("distributed_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
