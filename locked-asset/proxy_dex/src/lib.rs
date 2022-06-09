#![no_std]
#![feature(generic_associated_types)]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
pub mod migration_from_v1_2;
pub mod proxy_common;
pub mod proxy_farm;
mod proxy_pair;
mod wrapped_farm_token_merge;
mod wrapped_lp_token_merge;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub enum RegisterRequestType {
    ProxyFarm,
    ProxyPair,
}

#[elrond_wasm::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + proxy_farm::ProxyFarmModule
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
    + migration_from_v1_2::MigrationModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    ) {
        require!(
            asset_token_id.is_valid_esdt_identifier(),
            "Asset token ID is not a valid esdt identifier"
        );
        require!(
            locked_asset_token_id.is_valid_esdt_identifier(),
            "Locked asset token ID is not a valid esdt identifier"
        );
        require!(
            asset_token_id != locked_asset_token_id,
            "Locked asset token ID cannot be the same as Asset token ID"
        );

        self.asset_token_id().set(&asset_token_id);
        self.locked_asset_token_id().set(&locked_asset_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerProxyPair)]
    fn register_proxy_pair(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let register_cost = self.call_value().egld_value();
        self.wrapped_lp_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerProxyFarm)]
    fn register_proxy_farm(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let register_cost = self.call_value().egld_value();
        self.wrapped_farm_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }
}
