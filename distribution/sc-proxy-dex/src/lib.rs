#![no_std]
#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
mod proxy_common;
mod proxy_farm;
mod proxy_pair;
mod wrapped_farm_token_merge;
mod wrapped_lp_token_merge;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub enum IssueRequestType {
    ProxyFarm,
    ProxyPair,
}

#[elrond_wasm::contract]
pub trait ProxyDexImpl:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + proxy_farm::ProxyFarmModule
    + token_supply::TokenSupplyModule
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    ) -> SCResult<()> {
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
        Ok(())
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueSftProxyPair)]
    fn issue_sft_proxy_pair(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        #[payment_amount] issue_cost: BigUint,
    ) -> SCResult<AsyncCall> {
        require!(self.wrapped_lp_token_id().is_empty(), "SFT already issued");
        self.issue_nft(
            token_display_name,
            token_ticker,
            issue_cost,
            IssueRequestType::ProxyPair,
        )
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueSftProxyFarm)]
    fn issue_sft_proxy_farm(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        #[payment_amount] issue_cost: BigUint,
    ) -> SCResult<AsyncCall> {
        require!(
            self.wrapped_farm_token_id().is_empty(),
            "SFT already issued"
        );
        self.issue_nft(
            token_display_name,
            token_ticker,
            issue_cost,
            IssueRequestType::ProxyFarm,
        )
    }

    fn issue_nft(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        issue_cost: BigUint,
        request_type: IssueRequestType,
    ) -> SCResult<AsyncCall> {
        Ok(self
            .send()
            .esdt_system_sc_proxy()
            .issue_semi_fungible(
                issue_cost,
                &token_display_name,
                &token_ticker,
                SemiFungibleTokenProperties {
                    can_add_special_roles: true,
                    can_change_owner: false,
                    can_freeze: false,
                    can_pause: false,
                    can_upgrade: true,
                    can_wipe: false,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_nft_callback(request_type)))
    }

    #[callback]
    fn issue_nft_callback(
        &self,
        request_type: IssueRequestType,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.last_error_message().clear();

                match request_type {
                    IssueRequestType::ProxyPair => {
                        if self.wrapped_lp_token_id().is_empty() {
                            self.wrapped_lp_token_id().set(&token_id);
                        }
                    }
                    IssueRequestType::ProxyFarm => {
                        if self.wrapped_farm_token_id().is_empty() {
                            self.wrapped_farm_token_id().set(&token_id);
                        }
                    }
                }
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                let (payment, token_id) = self.call_value().payment_token_pair();
                self.send().direct(
                    &self.blockchain().get_owner_address(),
                    &token_id,
                    0,
                    &payment,
                    &[],
                );
            }
        };
    }

    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(
        &self,
        token: TokenIdentifier,
        address: ManagedAddress,
        #[var_args] roles: ManagedVarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall> {
        Ok(self
            .send()
            .esdt_system_sc_proxy()
            .set_special_roles(&address, &token, roles.into_iter())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;
}
