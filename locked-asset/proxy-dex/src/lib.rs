#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
mod proxy_common;
mod proxy_farm;
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
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    ) -> SCResult<()> {
        require!(
            asset_token_id.is_esdt(),
            "Asset token ID is not a valid esdt identifier"
        );
        require!(
            locked_asset_token_id.is_esdt(),
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
    #[endpoint(registerProxyPair)]
    fn register_proxy_pair(
        &self,
        #[payment_amount] register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) -> SCResult<AsyncCall> {
        require!(
            self.wrapped_lp_token_id().is_empty(),
            "Token exists already"
        );
        Ok(self.register_meta_esdt(
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            RegisterRequestType::ProxyPair,
        ))
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerProxyFarm)]
    fn register_proxy_farm(
        &self,
        #[payment_amount] register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) -> SCResult<AsyncCall> {
        require!(
            self.wrapped_farm_token_id().is_empty(),
            "Token exists already"
        );
        Ok(self.register_meta_esdt(
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            RegisterRequestType::ProxyFarm,
        ))
    }

    fn register_meta_esdt(
        &self,
        register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
        request_type: RegisterRequestType,
    ) -> AsyncCall {
        self.send()
            .esdt_system_sc_proxy()
            .register_meta_esdt(
                register_cost,
                &token_display_name,
                &token_ticker,
                MetaTokenProperties {
                    num_decimals,
                    can_add_special_roles: true,
                    can_change_owner: false,
                    can_freeze: false,
                    can_pause: false,
                    can_upgrade: true,
                    can_wipe: false,
                },
            )
            .async_call()
            .with_callback(self.callbacks().register_callback(request_type))
    }

    #[callback]
    fn register_callback(
        &self,
        request_type: RegisterRequestType,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.last_error_message().clear();

                match request_type {
                    RegisterRequestType::ProxyPair => {
                        if self.wrapped_lp_token_id().is_empty() {
                            self.wrapped_lp_token_id().set(&token_id);
                        }
                    }
                    RegisterRequestType::ProxyFarm => {
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
    ) -> AsyncCall {
        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&address, &token, roles.into_iter())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
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
