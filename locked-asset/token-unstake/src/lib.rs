#![no_std]

multiversx_sc::imports!();

pub mod cancel_unstake;
pub mod events;
pub mod fees_handler;
pub mod tokens_per_user;
pub mod unbond_tokens;

use crate::fees_handler::MAX_PENALTY_PERCENTAGE;

#[multiversx_sc::contract]
pub trait TokenUnstakeModule:
    tokens_per_user::TokensPerUserModule
    + unbond_tokens::UnbondTokensModule
    + cancel_unstake::CancelUnstakeModule
    + fees_handler::FeesHandlerModule
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
    + events::EventsModule
{
    /// Needs burn role for both the unlocked and locked token
    #[init]
    fn init(
        &self,
        unbond_epochs: u64,
        energy_factory_address: ManagedAddress,
        fees_burn_percentage: u64,
        fees_collector_address: ManagedAddress,
    ) {
        self.require_sc_address(&energy_factory_address);
        self.require_sc_address(&fees_collector_address);
        require!(
            fees_burn_percentage <= MAX_PENALTY_PERCENTAGE,
            "Invalid percentage"
        );

        self.unbond_epochs().set(unbond_epochs);
        self.energy_factory_address().set(&energy_factory_address);
        self.fees_collector_address().set(&fees_collector_address);
        self.fees_burn_percentage().set(fees_burn_percentage);
    }
}
