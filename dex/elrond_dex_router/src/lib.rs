#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod factory;
mod lp_tokens;
mod pair_manager;
mod util;

use factory::*;

const TRANSFER_EXEC_DEFAULT_GAS_LIMIT: u64 = 35_000_000;

#[elrond_wasm_derive::contract]
pub trait Router:
    factory::FactoryModule
    + pair_manager::PairManagerModule
    + lp_tokens::LpTokensModule
    + util::UtilModule
{
    #[init]
    fn init(&self) {
        self.state().set_if_empty(&true);
        self.pair_creation_enabled().set_if_empty(&false);
        self.transfer_exec_gas_limit()
            .set_if_empty(&TRANSFER_EXEC_DEFAULT_GAS_LIMIT);

        self.init_factory();
        self.owner().set(&self.blockchain().get_caller());
    }

    #[endpoint(createPairStable)]
    fn create_pair_stable(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> SCResult<Address> {
        self.create_pair_endpoint(
            first_token_id,
            second_token_id,
            STABLE_TOTAL_FEE_PERCENT,
            STABLE_SPECIAL_FEE_PERCENT,
        )
    }

    #[endpoint(createPairNormal)]
    fn create_pair_normal(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> SCResult<Address> {
        self.create_pair_endpoint(
            first_token_id,
            second_token_id,
            NORMAL_TOTAL_FEE_PERCENT,
            NORMAL_SPECIAL_FEE_PERCENT,
        )
    }

    #[endpoint(createPairExotic)]
    fn create_pair_exotic(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> SCResult<Address> {
        self.create_pair_endpoint(
            first_token_id,
            second_token_id,
            EXOTIC_TOTAL_FEE_PERCENT,
            EXOTIC_SPECIAL_FEE_PERCENT,
        )
    }

    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<Address> {
        require!(self.is_active(), "Not active");
        let owner = self.owner().get();
        let caller = self.blockchain().get_caller();
        require!(
            caller == owner || self.pair_creation_enabled().get(),
            "Pair creation is disabled"
        );
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
            "Second Token ID is not a valid esdt token ID"
        );
        let pair_address = self.get_pair(
            first_token_id.clone(),
            second_token_id.clone(),
            total_fee_percent,
            special_fee_percent,
        );
        require!(pair_address.is_none(), "Pair already exists");

        self.create_pair(
            &first_token_id,
            &second_token_id,
            &owner,
            total_fee_percent,
            special_fee_percent,
        )
    }

    #[endpoint]
    fn pause(&self, address: Address) -> SCResult<()> {
        self.require_owner()?;

        if address == self.blockchain().get_sc_address() {
            self.state().set(&false);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pause_pair(address);
        }
        Ok(())
    }

    #[endpoint]
    fn resume(&self, address: Address) -> SCResult<()> {
        self.require_owner()?;

        if address == self.blockchain().get_sc_address() {
            self.state().set(&true);
        } else {
            self.check_is_pair_sc(&address)?;
            self.resume_pair(address);
        }
        Ok(())
    }
}
