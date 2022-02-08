#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
pub mod factory;
pub mod lib;

use factory::PairTokens;
use pair::config::ProxyTrait as _;
use pair::fee::ProxyTrait as _;
use pair::ProxyTrait as _;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

const DEFAULT_TOTAL_FEE_PERCENT: u64 = 300;
const DEFAULT_SPECIAL_FEE_PERCENT: u64 = 50;
const MAX_TOTAL_FEE_PERCENT: u64 = 100_000;

#[elrond_wasm::contract]
pub trait Router:
    factory::FactoryModule + events::EventsModule + lib::Lib + token_send::TokenSendModule
{
    #[init]
    fn init(&self, #[var_args] pair_template_address_opt: OptionalArg<ManagedAddress>) {
        self.state().set_if_empty(&true);
        self.pair_creation_enabled().set_if_empty(&false);

        self.init_factory(pair_template_address_opt.into_option());
        self.owner().set(&self.blockchain().get_caller());
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&false);
        } else {
            self.check_is_pair_sc(&address);
            self.pair_contract_proxy(address)
                .pause()
                .execute_on_dest_context();
        }
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&true);
        } else {
            self.check_is_pair_sc(&address);
            self.pair_contract_proxy(address)
                .resume()
                .execute_on_dest_context();
        }
    }

    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        #[var_args] opt_fee_percents: OptionalArg<MultiArg2<u64, u64>>,
    ) -> ManagedAddress {
        require!(self.is_active(), "Not active");
        let owner = self.owner().get();
        let caller = self.blockchain().get_caller();

        if caller != owner {
            require!(
                self.pair_creation_enabled().get(),
                "Pair creation is disabled"
            );
        }

        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_esdt(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_esdt(),
            "Second Token ID is not a valid esdt token ID"
        );
        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(pair_address.is_zero(), "Pair already exists");

        let mut total_fee_percent_requested = DEFAULT_TOTAL_FEE_PERCENT;
        let mut special_fee_percent_requested = DEFAULT_SPECIAL_FEE_PERCENT;

        if caller == owner {
            if let Some(fee_percents_multi_arg) = opt_fee_percents.into_option() {
                let fee_percents_tuple = fee_percents_multi_arg.into_tuple();
                total_fee_percent_requested = fee_percents_tuple.0;
                special_fee_percent_requested = fee_percents_tuple.1;

                require!(
                    total_fee_percent_requested >= special_fee_percent_requested
                        && total_fee_percent_requested < MAX_TOTAL_FEE_PERCENT,
                    "Bad percents"
                );
            } else {
                sc_panic!("Bad percents length");
            }
        }

        let address = self.create_pair(
            &first_token_id,
            &second_token_id,
            &owner,
            total_fee_percent_requested,
            special_fee_percent_requested,
        );

        self.emit_create_pair_event(
            caller,
            first_token_id,
            second_token_id,
            total_fee_percent_requested,
            special_fee_percent_requested,
            address.clone(),
        );
        address
    }

    #[only_owner]
    #[endpoint(upgradePair)]
    fn upgrade_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        total_fee_percent_requested: u64,
        special_fee_percent_requested: u64,
    ) {
        require!(self.is_active(), "Not active");

        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_esdt(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_esdt(),
            "Second Token ID is not a valid esdt token ID"
        );
        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(!pair_address.is_zero(), "Pair does not exists");

        require!(
            total_fee_percent_requested >= special_fee_percent_requested
                && total_fee_percent_requested < MAX_TOTAL_FEE_PERCENT,
            "Bad percents"
        );

        self.upgrade_pair(
            &pair_address,
            &first_token_id,
            &second_token_id,
            &self.owner().get(),
            total_fee_percent_requested,
            special_fee_percent_requested,
        );
    }

    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        #[payment_amount] issue_cost: BigUint,
        pair_address: ManagedAddress,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) -> AsyncCall {
        require!(self.is_active(), "Not active");
        let caller = self.blockchain().get_caller();
        if caller != self.owner().get() {
            require!(
                self.pair_creation_enabled().get(),
                "Pair creation is disabled"
            );
        }
        self.check_is_pair_sc(&pair_address);
        let result = self.get_pair_temporary_owner(&pair_address);

        match result {
            None => {}
            Some(temporary_owner) => {
                require!(caller == temporary_owner, "Temporary owner differs");
            }
        };

        let result = self
            .pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(result.is_egld(), "LP Token already issued");

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost,
                &lp_token_display_name,
                &lp_token_ticker,
                &BigUint::from(LP_TOKEN_INITIAL_SUPPLY),
                FungibleTokenProperties {
                    num_decimals: LP_TOKEN_DECIMALS,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(
                self.callbacks()
                    .lp_token_issue_callback(&caller, &pair_address),
            )
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: ManagedAddress) -> AsyncCall {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address);

        let pair_token = self
            .pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(pair_token.is_esdt(), "LP token not issued");

        let roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&pair_address, &pair_token, roles.iter().cloned())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
    }

    #[only_owner]
    #[endpoint(setLocalRolesOwner)]
    fn set_local_roles_owner(
        &self,
        token: TokenIdentifier,
        address: ManagedAddress,
        #[var_args] roles: ManagedVarArgs<EsdtLocalRole>,
    ) -> AsyncCall {
        require!(self.is_active(), "Not active");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&address, &token, roles.into_iter())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
    }

    #[only_owner]
    #[endpoint(removePair)]
    fn remove_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> SCResult<ManagedAddress> {
        require!(self.is_active(), "Not active");

        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_esdt(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_esdt(),
            "Second Token ID is not a valid esdt token ID"
        );
        let mut pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(!pair_address.is_zero(), "Pair does not exists");

        pair_address = self
            .pair_map()
            .remove(&PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(ManagedAddress::zero);

        if pair_address.is_zero() {
            pair_address = self
                .pair_map()
                .remove(&PairTokens {
                    first_token_id: second_token_id,
                    second_token_id: first_token_id,
                })
                .unwrap_or_else(ManagedAddress::zero);
        }

        Ok(pair_address)
    }

    #[only_owner]
    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address);

        self.pair_contract_proxy(pair_address)
            .set_fee_on(true, fee_to_address, fee_token)
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address);

        self.pair_contract_proxy(pair_address)
            .set_fee_on(false, fee_to_address, fee_token)
            .execute_on_dest_context();
    }

    #[view(getPair)]
    fn get_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> ManagedAddress {
        let mut address = self
            .pair_map()
            .get(&PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(ManagedAddress::zero);

        if address.is_zero() {
            address = self
                .pair_map()
                .get(&PairTokens {
                    first_token_id: second_token_id,
                    second_token_id: first_token_id,
                })
                .unwrap_or_else(ManagedAddress::zero);
        }
        address
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] returned_tokens: BigUint,
        caller: &ManagedAddress,
        address: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<()>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.last_error_message().clear();

                self.pair_temporary_owner().remove(address);
                self.pair_contract_proxy(address.clone())
                    .set_lp_token_identifier(token_id)
                    .execute_on_dest_context();
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                if token_id.is_egld() && returned_tokens > 0u64 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
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

    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[only_owner]
    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) {
        self.pair_creation_enabled().set(&enabled);
    }

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<bool>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;
}
