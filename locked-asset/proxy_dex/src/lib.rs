#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod energy_update;
pub mod events;
pub mod external_merging;
pub mod farm_interactions;
pub mod pair_interactions;
pub mod proxy_common;
pub mod proxy_farm;
pub mod proxy_pair;
pub mod sc_whitelist;
pub mod wrapped_farm_attributes;
pub mod wrapped_farm_token_merge;
pub mod wrapped_lp_attributes;
pub mod wrapped_lp_token_merge;

#[multiversx_sc::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + sc_whitelist::ScWhitelistModule
    + proxy_pair::ProxyPairModule
    + pair_interactions::PairInteractionsModule
    + proxy_farm::ProxyFarmModule
    + farm_interactions::FarmInteractionsModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + energy_update::EnergyUpdateModule
    + energy_query::EnergyQueryModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    #[init]
    fn init(
        &self,
        old_locked_token_id: TokenIdentifier,
        old_factory_address: ManagedAddress,
        energy_factory_address: ManagedAddress,
    ) {
        self.require_valid_token_id(&old_locked_token_id);
        self.require_sc_address(&old_factory_address);
        self.require_sc_address(&energy_factory_address);

        self.old_locked_token_id()
            .set_if_empty(&old_locked_token_id);
        self.old_factory_address()
            .set_if_empty(&old_factory_address);
        self.energy_factory_address()
            .set_if_empty(&energy_factory_address);
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
        let register_cost = self.call_value().egld_value().clone_value();
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
    #[endpoint(setTransferRoleWrappedLpToken)]
    fn set_transfer_role_wrapped_lp_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.wrapped_lp_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
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
        let register_cost = self.call_value().egld_value().clone_value();
        self.wrapped_farm_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[only_owner]
    #[endpoint(setTransferRoleWrappedFarmToken)]
    fn set_transfer_role_wrapped_farm_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.wrapped_farm_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            None,
        );
    }
}
