#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
pub mod migration_from_v1_2;
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
    + migration_from_v1_2::MigrationModule
{
    #[init]
    fn init(&self) {}
}
