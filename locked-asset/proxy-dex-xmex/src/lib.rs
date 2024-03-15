#![no_std]

use common_structs::Epoch;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod create_pair_foundation;
pub mod create_pair_user;
pub mod proxy_interactions;

#[multiversx_sc::contract]
pub trait ProxyDexImpl:
proxy_dex::proxy_interactions::proxy_common::ProxyCommonModule
    + proxy_dex::other_sc_whitelist::OtherScWhitelistModule
    + proxy_interactions::proxy_pair::ProxyPairModule
    + proxy_dex::proxy_interactions::pair_interactions::PairInteractionsModule
    + proxy_interactions::proxy_farm::ProxyFarmModule
    + proxy_dex::proxy_interactions::farm_interactions::FarmInteractionsModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + proxy_dex::merge_tokens::wrapped_farm_token_merge::WrappedFarmTokenMerge
    + proxy_dex::merge_tokens::wrapped_lp_token_merge::WrappedLpTokenMerge
    + proxy_dex::energy_update::EnergyUpdateModule
    + energy_query::EnergyQueryModule
    + proxy_dex::events::EventsModule
    + create_pair_user::CreatePairUserModule
    + create_pair_foundation::CreatePairFoundationModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[init]
    fn init(
        &self,
        old_locked_token_id: TokenIdentifier,
        old_factory_address: ManagedAddress,
        energy_factory_address: ManagedAddress,
        foundation_address: ManagedAddress,
        lp_lock_epochs: Epoch,
    ) {
        self.require_valid_token_id(&old_locked_token_id);
        self.require_sc_address(&old_factory_address);
        self.require_sc_address(&energy_factory_address);

        self.old_locked_token_id().set(old_locked_token_id);
        self.old_factory_address().set(old_factory_address);
        self.energy_factory_address().set(energy_factory_address);
        self.set_foundation_address(foundation_address);
        self.set_lp_lock_epochs(lp_lock_epochs);
    }

    #[endpoint]
    fn upgrade(&self) {}

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
