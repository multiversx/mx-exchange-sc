#![no_std]

multiversx_sc::imports!();

pub mod config;
pub mod errors;
pub mod events;
pub mod external_interactions;
pub mod incentive;
pub mod views;
pub mod vote;

#[multiversx_sc::contract]
pub trait MEXGovernance:
    config::ConfigModule
    + events::EventsModule
    + external_interactions::farm_interactions::FarmInteractionsModule
    + external_interactions::energy_factory_interactions::EnergyFactoryInteractionsModule
    + incentive::IncentiveModule
    + vote::VoteModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + views::ViewsModule
{
    #[init]
    fn init(
        &self,
        reference_emission_rate: BigUint,
        incentive_token: TokenIdentifier,
        energy_factory_address: ManagedAddress,
    ) {
        self.set_reference_emission_rate(reference_emission_rate);
        self.set_incentive_token(incentive_token);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
