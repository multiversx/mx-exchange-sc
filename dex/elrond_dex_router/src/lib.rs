#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod events;
mod factory;
mod lp_tokens;
mod pair_manager;
mod state;

const DEFAULT_TOTAL_FEE_PERCENT: u64 = 300;
const DEFAULT_SPECIAL_FEE_PERCENT: u64 = 50;
const MAX_TOTAL_FEE_PERCENT: u64 = 100_000;

#[elrond_wasm::contract]
pub trait Router:
    factory::FactoryModule
    + pair_manager::PairManagerModule
    + lp_tokens::LpTokensModule
    + state::StateModule
    + events::EventsModule
    + token_send::TokenSendModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;

    #[init]
    fn init(&self) {
        self.state().set_if_empty(&true);
        self.pair_creation_enabled().set_if_empty(&false);

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
        require!(pair_address.is_none(), "Pair already exists");

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

        let address = self.create_pair(
            &first_token_id,
            &second_token_id,
            &owner,
            total_fee_percent_requested,
            special_fee_percent_requested,
        )?;

        self.emit_create_pair_event(
            caller,
            first_token_id,
            second_token_id,
            total_fee_percent_requested,
            special_fee_percent_requested,
            address.clone(),
        );
        Ok(address)
    }

    #[endpoint(upgradePair)]
    fn upgrade_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        #[var_args] fee_percents: VarArgs<u64>,
    ) -> SCResult<()> {
        only_owner!(self, "No permissions");
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
        require!(pair_address.is_some(), "Pair does not exists");

        let fee_percents_vec = fee_percents.into_vec();
        require!(fee_percents_vec.len() == 2, "Bad percents length");

        let total_fee_percent_requested = fee_percents_vec[0];
        let special_fee_percent_requested = fee_percents_vec[1];
        require!(
            total_fee_percent_requested >= special_fee_percent_requested
                && total_fee_percent_requested < MAX_TOTAL_FEE_PERCENT,
            "Bad percents"
        );

        self.upgrade_pair(
            &pair_address.unwrap(),
            &first_token_id,
            &second_token_id,
            &self.owner().get(),
            total_fee_percent_requested,
            special_fee_percent_requested,
        )
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self, address: Address) -> SCResult<()> {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&false);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pause_pair(address);
        }
        Ok(())
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self, address: Address) -> SCResult<()> {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&true);
        } else {
            self.check_is_pair_sc(&address)?;
            self.resume_pair(address);
        }
        Ok(())
    }
}
