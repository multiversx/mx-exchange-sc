#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod events;
pub mod proxy_common;
pub mod proxy_farm;
pub mod proxy_pair;
pub mod wrapped_farm_attributes;
pub mod wrapped_farm_token_merge;
pub mod wrapped_lp_attributes;
pub mod wrapped_lp_token_merge;

#[elrond_wasm::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + proxy_farm::ProxyFarmModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
{
    /// asset_token_id: underlying asset token ID, which is used to interact with
    ///     pair/farm contracts
    ///
    /// locked_token_factory_address_pairs: pairs of (token ID, address)
    ///     token_id: the LOCKED token ID that is generated by the given factory address
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_token_factory_address_pairs: MultiValueEncoded<
            MultiValue2<TokenIdentifier, ManagedAddress>,
        >,
    ) {
        self.require_valid_token_id(&asset_token_id);

        for arg_pair in locked_token_factory_address_pairs {
            let (locked_token_id, factory_address) = arg_pair.into_tuple();
            self.require_valid_token_id(&locked_token_id);
            require!(
                asset_token_id != locked_token_id,
                "Locked asset token ID cannot be the same as Asset token ID"
            );
            self.require_sc_address(&factory_address);

            self.factory_address_for_locked_token(&locked_token_id)
                .set(&factory_address);

            let is_new = self.locked_token_ids().insert(locked_token_id);
            require!(is_new, "Locked token already assigned to another address");
        }
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
