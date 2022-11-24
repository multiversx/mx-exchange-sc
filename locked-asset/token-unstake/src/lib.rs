#![no_std]

elrond_wasm::imports!();

pub mod cancel_unstake;
pub mod events;
pub mod fees_accumulation;
pub mod tokens_per_user;
pub mod unbond_tokens;

use common_structs::{Epoch, Percent};
use energy_factory::lock_options::{AllLockOptions, LockOption, MAX_PENALTY_PERCENTAGE};

#[elrond_wasm::contract]
pub trait TokenUnstakeModule:
    tokens_per_user::TokensPerUserModule
    + unbond_tokens::UnbondTokensModule
    + cancel_unstake::CancelUnstakeModule
    + fees_accumulation::FeesAccumulationModule
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
    + energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
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
        lock_options: MultiValueEncoded<MultiValue2<Epoch, Percent>>,
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

        // TODO: See if we can get this from energy factory here
        let mut options = AllLockOptions::new();
        for pair in lock_options {
            let (lock_epochs, penalty_start_percentage) = pair.into_tuple();
            unsafe {
                options.push_unchecked(LockOption {
                    lock_epochs,
                    penalty_start_percentage,
                });
            }
        }
        self.lock_options().set(&options);
    }
}
