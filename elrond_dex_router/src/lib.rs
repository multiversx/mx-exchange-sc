#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod factory;
pub use factory::*;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
    fn set_fee_on(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> ContractCall<BigUint, ()>;
    fn set_lp_token_identifier(
        &self,
        token_identifier: TokenIdentifier,
    ) -> ContractCall<BigUint, ()>;
    fn get_lp_token_identifier(&self) -> ContractCall<BigUint, TokenIdentifier>;
    fn pause(&self) -> ContractCall<BigUint, ()>;
    fn resume(&self) -> ContractCall<BigUint, ()>;
}

#[elrond_wasm_derive::callable(StakingContractProxy)]
pub trait StakingContract {
    fn add_pair(&self, address: Address, token: TokenIdentifier) -> ContractCall<BigUint, ()>;
    fn remove_pair(&self, address: Address, token: TokenIdentifier) -> ContractCall<BigUint, ()>;
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
    }

    #[endpoint]
    fn pause(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        if address == self.get_sc_address() {
            self.state().set(&false);
        } else if !self.staking_address().is_empty() && address == self.staking_address().get() {
            contract_call!(self, address, StakingContractProxy)
                .pause()
                .execute_on_dest_context(self.get_gas_left(), self.send());
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
        } else if !self.staking_address().is_empty() && address == self.staking_address().get() {
            contract_call!(self, address, StakingContractProxy)
                .resume()
                .execute_on_dest_context(self.get_gas_left(), self.send());
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
    ) -> SCResult<Address> {
        require!(self.is_active(), "Not active");
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(first_token_id.is_esdt(), "Only esdt tokens allowed");
        require!(second_token_id.is_esdt(), "Only esdt tokens allowed");
        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(pair_address == Address::zero(), "Pair already existent");
        Ok(self
            .factory()
            .create_pair(&first_token_id, &second_token_id))
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
            .get_lp_token_identifier()
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
            .get_lp_token_identifier()
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

    #[endpoint(setStakingInfo)]
    fn set_staking_info(
        &self,
        staking_address: Address,
        staking_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        self.staking_address().set(&staking_address);
        self.staking_token().set(&staking_token);
        Ok(())
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
    fn set_fee_on(&self, pair_address: Address) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        sc_try!(self.check_is_pair_sc(&pair_address));
        require!(!self.staking_address().is_empty(), "Empty staking address");
        require!(!self.staking_token().is_empty(), "Empty staking token");

        let per_execute_gas = self.get_gas_left() / 3;
        let staking_token = self.staking_token().get();
        let staking_address = self.staking_address().get();
        contract_call!(self, pair_address.clone(), PairContractProxy)
            .set_fee_on(true, staking_address, staking_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        let lp_token = contract_call!(self, pair_address.clone(), PairContractProxy)
            .get_lp_token_identifier()
            .execute_on_dest_context(per_execute_gas, self.send());

        contract_call!(self, self.staking_address().get(), StakingContractProxy)
            .add_pair(pair_address, lp_token)
            .execute_on_dest_context(per_execute_gas, self.send());

        Ok(())
    }

    #[endpoint(setFeeOff)]
    fn set_fee_off(&self, pair_address: Address) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        sc_try!(self.check_is_pair_sc(&pair_address));
        require!(!self.staking_address().is_empty(), "Empty staking address");

        let per_execute_gas = self.get_gas_left() / 3;
        contract_call!(self, pair_address.clone(), PairContractProxy)
            .set_fee_on(false, Address::zero(), TokenIdentifier::egld())
            .execute_on_dest_context(per_execute_gas, self.send());

        let lp_token = contract_call!(self, pair_address.clone(), PairContractProxy)
            .get_lp_token_identifier()
            .execute_on_dest_context(per_execute_gas, self.send());

        contract_call!(self, self.staking_address().get(), StakingContractProxy)
            .remove_pair(pair_address, lp_token)
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

    #[view(getPair)]
    fn get_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> Address {
        let mut address = self
            .factory()
            .pair_map()
            .get(&PairKey {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(Address::zero);
        if address == Address::zero() {
            address = self
                .factory()
                .pair_map()
                .get(&PairKey {
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
        #[call_result] result: AsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            AsyncCallResult::Ok(token_id) => {
                contract_call!(self, address.clone(), PairContractProxy)
                    .set_lp_token_identifier(token_id)
                    .execute_on_dest_context(self.get_gas_left(), self.send());
            }
            AsyncCallResult::Err(_) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
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

    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[view(getStakingAddress)]
    #[storage_mapper("staking_address")]
    fn staking_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getStakingToken)]
    #[storage_mapper("staking_token")]
    fn staking_token(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;
}
