#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod factory;
mod lp_tokens;
mod pair_manager;
mod util;

const DEFAULT_TOTAL_FEE_PERCENT: u64 = 300;
const DEFAULT_SPECIAL_FEE_PERCENT: u64 = 50;
const MAX_TOTAL_FEE_PERCENT: u64 = 100_000;

const TRANSFER_EXEC_DEFAULT_GAS_LIMIT: u64 = 100_000;

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
