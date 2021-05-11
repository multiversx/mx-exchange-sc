elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use dex_common::*;

#[elrond_wasm_derive::module]
pub trait FeeModule {
    #[storage_mapper("fee_destination")]
    fn destination_map(&self) -> MapMapper<Self::Storage, Address, TokenIdentifier>;

    #[storage_mapper("trusted_swap_pair")]
    fn trusted_swap_pair(&self) -> MapMapper<Self::Storage, TokenPair, Address>;

    #[view(getWhitelistedAddresses)]
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getFeeState)]
    fn is_enabled(&self) -> bool {
        !self.destination_map().is_empty()
    }
}
