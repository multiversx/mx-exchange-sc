#![no_std]

multiversx_sc::imports!();

pub mod cancel_unstake;
pub mod events;
pub mod fees_handler;
pub mod tokens_per_user;
pub mod unbond_tokens;

use common_structs::{Epoch, Percent};

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
        unbond_epochs: Epoch,
        energy_factory_address: ManagedAddress,
        fees_burn_percentage: Percent,
        fees_collector_address: ManagedAddress,
    ) {
        self.require_sc_address(&fees_collector_address);

        self.set_energy_factory_address(energy_factory_address);
        self.set_fees_burn_percent(fees_burn_percentage);

        self.unbond_epochs().set(unbond_epochs);
        self.fees_collector_address().set(fees_collector_address);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
