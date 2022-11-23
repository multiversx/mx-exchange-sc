#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
pub mod proxy_common;
pub mod proxy_farm;
mod proxy_pair;

#[elrond_wasm::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + proxy_farm::ProxyFarmModule
    + token_merge::TokenMergeModule
    + events::EventsModule
{
    #[init]
    fn init(&self) {}
}
