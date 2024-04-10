#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
pub mod enable_swap_by_user;
mod events;
pub mod factory;
pub mod multi_pair_swap;

use factory::PairTokens;
use pair::config::ProxyTrait as _;
use pair::fee::ProxyTrait as _;
use pair::ProxyTrait as _;
use pausable::ProxyTrait as _;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

pub const DEFAULT_TOTAL_FEE_PERCENT: u64 = 300;
pub const DEFAULT_SPECIAL_FEE_PERCENT: u64 = 50;
const MAX_TOTAL_FEE_PERCENT: u64 = 100_000;
const USER_DEFINED_TOTAL_FEE_PERCENT: u64 = 1_000;

#[multiversx_sc::contract]
pub trait Router:
    config::ConfigModule
    + factory::FactoryModule
    + events::EventsModule
    + multi_pair_swap::MultiPairSwap
    + token_send::TokenSendModule
    + enable_swap_by_user::EnableSwapByUserModule
{
    #[init]
    fn init(&self, pair_template_address_opt: OptionalValue<ManagedAddress>) {
        self.state().set_if_empty(true);
        self.pair_creation_enabled().set_if_empty(false);

        self.init_factory(pair_template_address_opt.into_option());
        self.owner().set(&self.blockchain().get_caller());
    }

    #[endpoint]
    fn upgrade(&self) {
        self.state().set(false);
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            self.state().set(false);
        } else {
            self.check_is_pair_sc(&address);
            let _: IgnoreValue = self
                .pair_contract_proxy(address)
                .pause()
                .execute_on_dest_context();
        }
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            require!(
                self.pair_map().len() == self.address_pair_map().len(),
                "The size of the 2 pair maps is not the same"
            );
            self.state().set(true);
        } else {
            self.check_is_pair_sc(&address);
            let _: IgnoreValue = self
                .pair_contract_proxy(address)
                .resume()
                .execute_on_dest_context();
        }
    }

    #[allow_multiple_var_args]
    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        initial_liquidity_adder: ManagedAddress,
        opt_fee_percents: OptionalValue<MultiValue2<u64, u64>>,
        mut admins: MultiValueEncoded<ManagedAddress>,
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
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
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

        admins.push(caller.clone());

        let address = self.create_pair(
            &first_token_id,
            &second_token_id,
            &owner,
            total_fee_percent_requested,
            special_fee_percent_requested,
            &initial_liquidity_adder,
            admins,
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
        initial_liquidity_adder: ManagedAddress,
        total_fee_percent_requested: u64,
        special_fee_percent_requested: u64,
    ) {
        require!(self.is_active(), "Not active");
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
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
            pair_address,
            &first_token_id,
            &second_token_id,
            &self.owner().get(),
            &initial_liquidity_adder,
            total_fee_percent_requested,
            special_fee_percent_requested,
        );
    }

    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        pair_address: ManagedAddress,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address);

        let issue_cost = self.call_value().egld_value().clone_value();
        let caller = self.blockchain().get_caller();
        if caller != self.owner().get() {
            require!(
                self.pair_creation_enabled().get(),
                "Pair creation is disabled"
            );
        }

        let opt_temp_owner = self.get_pair_temporary_owner(&pair_address);
        if let Some(temporary_owner) = opt_temp_owner {
            require!(caller == temporary_owner, "Temporary owner differs");
        }

        let lp_token_id: TokenIdentifier = self
            .pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(
            !lp_token_id.is_valid_esdt_identifier(),
            "LP Token already issued"
        );

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
            .call_and_exit();
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: ManagedAddress) {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address);

        let pair_token: TokenIdentifier = self
            .pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(pair_token.is_valid_esdt_identifier(), "LP token not issued");

        let roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&pair_address, &pair_token, roles.iter().cloned())
            .async_call()
            .call_and_exit();
    }

    #[only_owner]
    #[endpoint(setLocalRolesOwner)]
    fn set_local_roles_owner(
        &self,
        token: TokenIdentifier,
        address: ManagedAddress,
        roles: MultiValueEncoded<EsdtLocalRole>,
    ) {
        require!(self.is_active(), "Not active");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&address, &token, roles.into_iter())
            .async_call()
            .call_and_exit()
    }

    #[only_owner]
    #[endpoint(removePair)]
    fn remove_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> ManagedAddress {
        require!(self.is_active(), "Not active");

        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
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

        self.address_pair_map().remove(&pair_address);

        pair_address
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

        let _: IgnoreValue = self
            .pair_contract_proxy(pair_address)
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

        let _: IgnoreValue = self
            .pair_contract_proxy(pair_address)
            .set_fee_on(false, fee_to_address, fee_token)
            .execute_on_dest_context();
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: &ManagedAddress,
        address: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<()>,
    ) {
        let (token_id, returned_tokens) = self.call_value().egld_or_single_fungible_esdt();
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.pair_temporary_owner().remove(address);
                let _: IgnoreValue = self
                    .pair_contract_proxy(address.clone())
                    .set_lp_token_identifier(token_id.unwrap_esdt())
                    .execute_on_dest_context();
            }
            ManagedAsyncCallResult::Err(_) => {
                if token_id.is_egld() && returned_tokens > 0u64 {
                    self.send().direct_egld(caller, &returned_tokens);
                }
            }
        }
    }

    #[only_owner]
    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) {
        self.pair_creation_enabled().set(enabled);
    }

    #[only_owner]
    #[endpoint(migratePairMap)]
    fn migrate_pair_map(
        &self,
        token_pairs: MultiValueEncoded<MultiValue2<TokenIdentifier, TokenIdentifier>>,
    ) {
        let pair_map = self.pair_map();
        let mut address_pair_map = self.address_pair_map();
        for token_pair_values in token_pairs {
            let (first_token_id, second_token_id) = token_pair_values.into_tuple();
            let pair_tokens = PairTokens {
                first_token_id,
                second_token_id,
            };
            let lp_address_opt = pair_map.get(&pair_tokens);
            require!(lp_address_opt.is_some(), "LP address not found");
            unsafe {
                let lp_address = lp_address_opt.unwrap_unchecked();
                require!(
                    !address_pair_map.contains_key(&lp_address),
                    "Address pair mapper already contains these values"
                );
                address_pair_map.insert(lp_address, pair_tokens);
            }
        }
    }

    #[only_owner]
    #[endpoint(claimDeveloperRewardsPairs)]
    fn claim_developer_rewards_pairs(&self, pairs: MultiValueEncoded<ManagedAddress>) {
        let mut total_egld_received = BigUint::zero();
        for pair in pairs {
            let (_return_value, transfers): (IgnoreValue, _) = self
                .send()
                .claim_developer_rewards(pair)
                .execute_on_dest_context_with_back_transfers();

            total_egld_received += transfers.total_egld_amount;
        }

        let owner = self.blockchain().get_caller();
        self.send().direct_egld(&owner, &total_egld_received);
    }
}
