elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenPair {
    pub first_token: TokenIdentifier,
    pub second_token: TokenIdentifier,
}

#[elrond_wasm_derive::module(FeeModuleImpl)]
pub trait FeeModule {
    #[storage_mapper("fee_destination")]
    fn destination_map(&self) -> MapMapper<Self::Storage, Address, TokenIdentifier>;

    #[storage_mapper("pair_address_cache")]
    fn pair_address_cache_map(&self) -> MapMapper<Self::Storage, TokenPair, Address>;

    #[view(isWhiteListed)]
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getFeeState)]
    fn is_enabled(&self) -> bool {
        !self.destination_map().is_empty()
    }
}
