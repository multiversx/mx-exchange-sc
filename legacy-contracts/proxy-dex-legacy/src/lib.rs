#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod energy_update;
mod events;
pub mod proxy_common;
pub mod proxy_farm;
mod proxy_pair;
pub mod transfer_role;

use factory_legacy::attr_ex_helper;

#[multiversx_sc::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + proxy_farm::ProxyFarmModule
    + token_merge_helper::TokenMergeHelperModule
    + events::EventsModule
    + energy_update::EnergyUpdateModule
    + transfer_role::TransferRoleModule
    + attr_ex_helper::AttrExHelper
    + energy_query::EnergyQueryModule
{
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}
}
