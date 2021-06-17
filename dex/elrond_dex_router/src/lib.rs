#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod factory;
use factory::PairTokens;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

const DEFAULT_TOTAL_FEE_PERCENT: u64 = 300;
const DEFAULT_SPECIAL_FEE_PERCENT: u64 = 50;
const MAX_TOTAL_FEE_PERCENT: u64 = 100_000;

#[elrond_wasm_derive::contract]
pub trait Router: factory::FactoryModule {
    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;

    #[init]
    fn init(&self) {
        self.state().set_if_empty(&true);
        self.pair_creation_enabled().set_if_empty(&false);

        self.init_factory();
        self.owner().set(&self.blockchain().get_caller());
    }

    #[endpoint]
    fn pause(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        if address == self.blockchain().get_sc_address() {
            self.state().set(&false);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pair_contract_proxy(address)
                .pause()
                .execute_on_dest_context();
        }
        Ok(())
    }

    #[endpoint]
    fn resume(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        if address == self.blockchain().get_sc_address() {
            self.state().set(&true);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pair_contract_proxy(address)
                .resume()
                .execute_on_dest_context();
        }
        Ok(())
    }

    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        #[var_args] fee_percents: VarArgs<u64>,
    ) -> SCResult<Address> {
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
        require!(pair_address == Address::zero(), "Pair already exists");

        let mut total_fee_percent_requested = DEFAULT_TOTAL_FEE_PERCENT;
        let mut special_fee_percent_requested = DEFAULT_SPECIAL_FEE_PERCENT;
        let fee_percents_vec = fee_percents.into_vec();

        if caller == owner {
            require!(fee_percents_vec.len() == 2, "Bad percents length");
            total_fee_percent_requested = fee_percents_vec[0];
            special_fee_percent_requested = fee_percents_vec[1];
            require!(
                total_fee_percent_requested >= special_fee_percent_requested
                    && total_fee_percent_requested < MAX_TOTAL_FEE_PERCENT,
                "Bad percents"
            );
        }

        self.create_pair(
            &first_token_id,
            &second_token_id,
            &owner,
            total_fee_percent_requested,
            special_fee_percent_requested,
        )
    }

    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        pair_address: Address,
        tp_token_display_name: BoxedBytes,
        tp_token_ticker: BoxedBytes,
        #[payment_amount] issue_cost: Self::BigUint,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        let caller = self.blockchain().get_caller();
        if caller != self.owner().get() {
            require!(
                self.pair_creation_enabled().get(),
                "Pair creation is disabled"
            );
        }
        self.check_is_pair_sc(&pair_address)?;
        let result = self.get_pair_temporary_owner(&pair_address);

        match result {
            None => {}
            Some(temporary_owner) => {
                require!(caller == temporary_owner, "Temporary owner differs");
            }
        };

        let result = self
            .pair_contract_proxy(pair_address.clone())
            .getLpTokenIdentifier()
            .execute_on_dest_context();
        require!(result.is_egld(), "LP Token already issued");

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .issue_fungible(
                issue_cost,
                &tp_token_display_name,
                &tp_token_ticker,
                &Self::BigUint::from(LP_TOKEN_INITIAL_SUPPLY),
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
            ))
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: Address) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address)?;

        let pair_token = self
            .pair_contract_proxy(pair_address.clone())
            .getLpTokenIdentifier()
            .execute_on_dest_context();
        require!(pair_token.is_esdt(), "LP token not issued");

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(
                &pair_address,
                &pair_token,
                &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    #[endpoint(setLocalRolesOwner)]
    fn set_local_roles_owner(
        &self,
        token: TokenIdentifier,
        address: Address,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "No permissions");
        require!(!roles.is_empty(), "Empty roles");
        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(&address, &token, roles.as_slice())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    fn check_is_pair_sc(&self, pair_address: &Address) -> SCResult<()> {
        require!(
            self.pair_map()
                .values()
                .any(|address| &address == pair_address),
            "Not a pair SC"
        );
        Ok(())
    }

    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: Address,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .setFeeOn(true, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: Address,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .setFeeOn(false, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    #[endpoint(startPairCodeConstruction)]
    fn start_pair_code_construction(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.start_pair_construct();
        Ok(())
    }

    #[endpoint(endPairCodeConstruction)]
    fn end_pair_code_construction(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.end_pair_construct();
        Ok(())
    }

    #[endpoint(appendPairCode)]
    fn apppend_pair_code(&self, part: BoxedBytes) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.append_pair_code(&part)
    }

    #[view(getPair)]
    fn get_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> Address {
        let mut address = self
            .pair_map()
            .get(&PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(Address::zero);
        if address == Address::zero() {
            address = self
                .pair_map()
                .get(&PairTokens {
                    first_token_id: second_token_id,
                    second_token_id: first_token_id,
                })
                .unwrap_or_else(Address::zero);
        }
        address
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: &Address,
        address: &Address,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] returned_tokens: Self::BigUint,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.last_error_message().clear();

                self.pair_temporary_owner().remove(address);
                self.pair_contract_proxy(address.clone())
                    .setLpTokenIdentifier(token_id)
                    .execute_on_dest_context();
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: AsyncCallResult<()>) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.pair_creation_enabled().set(&enabled);
        Ok(())
    }

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<Self::Storage, Address>;
}
