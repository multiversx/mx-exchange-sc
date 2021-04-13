#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod factory;
pub use factory::*;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

const DEFAULT_TOTAL_FEE_PRECENT: u64 = 3;
const DEFAULT_SPECIAL_FEE_PRECENT: u64 = 1;

#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
    fn setFeeOn(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> ContractCall<BigUint, ()>;
    fn setLpTokenIdentifier(&self, token_identifier: TokenIdentifier) -> ContractCall<BigUint, ()>;
    fn getLpTokenIdentifier(&self) -> ContractCall<BigUint, TokenIdentifier>;
    fn pause(&self) -> ContractCall<BigUint, ()>;
    fn resume(&self) -> ContractCall<BigUint, ()>;
    fn whitelist(&self, address: Address) -> ContractCall<BigUint, ()>;
}

#[elrond_wasm_derive::callable(StakingContractProxy)]
pub trait StakingContract {
    fn addPair(&self, address: Address, token: TokenIdentifier) -> ContractCall<BigUint, ()>;
    fn removePair(&self, address: Address, token: TokenIdentifier) -> ContractCall<BigUint, ()>;
    fn pause(&self) -> ContractCall<BigUint, ()>;
    fn resume(&self) -> ContractCall<BigUint, ()>;
}

#[elrond_wasm_derive::contract(RouterImpl)]
pub trait Router {
    #[module(FactoryModuleImpl)]
    fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

    #[init]
    fn init(&self) {
        self.factory().init();
        self.state().set(&true);
        self.owner().set(&self.get_caller());
    }

    #[endpoint]
    fn pause(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        if address == self.get_sc_address() {
            self.state().set(&false);
        } else {
            sc_try!(self.check_is_pair_sc(&address));
            contract_call!(self, address, PairContractProxy)
                .pause()
                .execute_on_dest_context(self.get_gas_left(), self.send());
        }
        Ok(())
    }

    #[endpoint]
    fn resume(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        if address == self.get_sc_address() {
            self.state().set(&true);
        } else {
            sc_try!(self.check_is_pair_sc(&address));
            contract_call!(self, address, PairContractProxy)
                .resume()
                .execute_on_dest_context(self.get_gas_left(), self.send());
        }
        Ok(())
    }

    //ENDPOINTS
    #[endpoint(createPair)]
    fn create_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        #[var_args] fee_precents: VarArgs<u64>,
    ) -> SCResult<Address> {
        require!(self.is_active(), "Not active");
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(first_token_id.is_esdt(), "Only esdt tokens allowed");
        require!(second_token_id.is_esdt(), "Only esdt tokens allowed");
        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(pair_address == Address::zero(), "Pair already exists");
        let mut total_fee_precent_requested = DEFAULT_TOTAL_FEE_PRECENT;
        let mut special_fee_precent_requested = DEFAULT_SPECIAL_FEE_PRECENT;
        let fee_precents_vec = fee_precents.0;
        if self.get_caller() == self.owner().get() && fee_precents_vec.len() == 2 {
            total_fee_precent_requested = fee_precents_vec[0];
            special_fee_precent_requested = fee_precents_vec[1];
            require!(
                total_fee_precent_requested >= special_fee_precent_requested,
                "Bad precents"
            );
            (total_fee_precent_requested, special_fee_precent_requested)
        } else {
            (DEFAULT_TOTAL_FEE_PRECENT, DEFAULT_SPECIAL_FEE_PRECENT)
        };
        Ok(self.factory().create_pair(
            &first_token_id,
            &second_token_id,
            total_fee_precent_requested,
            special_fee_precent_requested,
        ))
    }

    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        pair_address: Address,
        tp_token_display_name: BoxedBytes,
        tp_token_ticker: BoxedBytes,
        #[payment] issue_cost: BigUint,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        sc_try!(self.check_is_pair_sc(&pair_address));

        let half_gas = self.get_gas_left() / 2;
        let result = contract_call!(self, pair_address.clone(), PairContractProxy)
            .getLpTokenIdentifier()
            .execute_on_dest_context(half_gas, self.send());

        require!(result.is_egld(), "LP Token already issued");

        Ok(ESDTSystemSmartContractProxy::new()
            .issue_fungible(
                issue_cost,
                &tp_token_display_name,
                &tp_token_ticker,
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
                    .lp_token_issue_callback(&self.get_caller(), &pair_address),
            ))
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: Address) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        sc_try!(self.check_is_pair_sc(&pair_address));

        let half_gas = self.get_gas_left() / 2;
        let pair_token = contract_call!(self, pair_address.clone(), PairContractProxy)
            .getLpTokenIdentifier()
            .execute_on_dest_context(half_gas, self.send());
        require!(pair_token.is_esdt(), "LP token not issued");

        Ok(ESDTSystemSmartContractProxy::new()
            .set_special_roles(
                &pair_address,
                pair_token.as_esdt_identifier(),
                &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    fn check_is_pair_sc(&self, pair_address: &Address) -> SCResult<()> {
        require!(
            self.factory()
                .pair_map()
                .values()
                .any(|address| &address == pair_address),
            "Not a pair SC"
        );
        Ok(())
    }

    #[endpoint(upgradePair)]
    fn upgrade_pair(&self, pair_address: Address) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        sc_try!(self.check_is_pair_sc(&pair_address));

        self.factory().upgrade_pair(&pair_address);
        Ok(())
    }

    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: Address,
        staking_address: Address,
        staking_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        sc_try!(self.check_is_pair_sc(&pair_address));

        let per_execute_gas = self.get_gas_left() / 3;
        contract_call!(self, pair_address.clone(), PairContractProxy)
            .setFeeOn(true, staking_address.clone(), staking_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        let lp_token = contract_call!(self, pair_address.clone(), PairContractProxy)
            .getLpTokenIdentifier()
            .execute_on_dest_context(per_execute_gas, self.send());

        contract_call!(self, staking_address, StakingContractProxy)
            .addPair(pair_address, lp_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        Ok(())
    }

    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: Address,
        staking_address: Address,
        staking_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        sc_try!(self.check_is_pair_sc(&pair_address));

        let per_execute_gas = self.get_gas_left() / 3;
        contract_call!(self, pair_address.clone(), PairContractProxy)
            .setFeeOn(false, staking_address.clone(), staking_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        let lp_token = contract_call!(self, pair_address.clone(), PairContractProxy)
            .getLpTokenIdentifier()
            .execute_on_dest_context(per_execute_gas, self.send());

        contract_call!(self, staking_address, StakingContractProxy)
            .removePair(pair_address, lp_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        Ok(())
    }

    #[endpoint(startPairCodeConstruction)]
    fn start_pair_code_construction(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.factory().start_pair_construct();
        Ok(())
    }

    #[endpoint(endPairCodeConstruction)]
    fn end_pair_code_construction(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.factory().end_pair_construct();
        Ok(())
    }

    #[endpoint(appendPairCode)]
    fn apppend_pair_code(&self, part: BoxedBytes) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");

        self.factory().append_pair_code(&part);
        Ok(())
    }

    #[endpoint(getPairAndWhitelist)]
    fn get_pair_and_whitelist(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> Address {
        let caller = self.get_caller();
        let caller_is_pair_sc = self
            .factory()
            .pair_map()
            .values()
            .any(|address| address == caller);

        let zero_address = Address::zero();
        let req_address = if caller_is_pair_sc {
            self.get_pair(first_token_id, second_token_id)
        } else {
            zero_address.clone()
        };

        if req_address != zero_address {
            let half_gas = self.get_gas_left() * 4 / 5;
            contract_call!(self, req_address.clone(), PairContractProxy)
                .whitelist(caller)
                .execute_on_dest_context(half_gas, self.send());
        }

        req_address
    }

    #[view(getPair)]
    fn get_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> Address {
        let mut address = self
            .factory()
            .pair_map()
            .get(&PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(Address::zero);
        if address == Address::zero() {
            address = self
                .factory()
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
        #[payment] returned_tokens: BigUint,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        // let (returned_tokens, token_id) = self.call_value().payment_token_pair();
        match result {
            AsyncCallResult::Ok(()) => {
                contract_call!(self, address.clone(), PairContractProxy)
                    .setLpTokenIdentifier(token_id)
                    .execute_on_dest_context(self.get_gas_left(), self.send());
            }
            AsyncCallResult::Err(_) => {
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send().direct_egld(caller, &returned_tokens, &[]);
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
