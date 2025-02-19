#![no_std]

multiversx_sc::imports!();

pub mod config;
pub mod errors;
pub mod events;
pub mod views;

use config::FarmEmission;
use errors::*;

#[multiversx_sc::contract]
pub trait MEXGovernance:
    config::ConfigModule
    + events::EventsModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + views::ViewsModule
{
    #[init]
    fn init(&self, reference_emission_rate: BigUint, energy_factory_address: ManagedAddress) {
        self.set_reference_emission_rate(reference_emission_rate);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(vote)]
    fn vote(&self, votes: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>) {
        let caller = self.blockchain().get_caller();
        let current_week = self.get_current_week();
        let voting_week = current_week + 1;

        self.advance_week_if_needed(voting_week);

        require!(
            !self.users_voted_in_week(voting_week).contains(&caller),
            ALREADY_VOTED_THIS_WEEK
        );

        let user_energy = self.get_energy_amount_non_zero(&caller);

        let mut total_vote_amount = BigUint::zero();
        let mut farm_votes = ManagedVec::new();
        for vote in votes {
            let (farm_address, amount) = vote.into_tuple();
            require!(
                self.whitelisted_farms().contains(&farm_address),
                FARM_ADDRESS_NOT_WHITELISTED
            );
            require!(
                !self.blacklisted_farms().contains(&farm_address),
                FARM_BLACKLISTED
            );

            self.farm_votes_for_week(&farm_address, voting_week)
                .update(|sum| *sum += amount.clone());
            self.voted_farms_for_week(voting_week)
                .insert(farm_address.clone());

            total_vote_amount += &amount;

            farm_votes.push(FarmEmission {
                farm_address,
                farm_emission: amount,
            });
        }

        require!(total_vote_amount == user_energy, INVALID_VOTE_AMOUNT);

        self.total_energy_voted(voting_week)
            .update(|sum| *sum += &user_energy);
        self.users_voted_in_week(voting_week).add(&caller);

        self.emit_vote_event(voting_week, farm_votes);
    }

    fn advance_week_if_needed(&self, voting_week: usize) {
        let saved_week = self.voting_week().get();
        if saved_week < voting_week {
            self.voting_week().set(voting_week);

            let emission_rate = self.reference_emission_rate().get();
            self.emission_rate_for_week(voting_week).set(emission_rate);
        }
    }

    // TODO - to implement
    // fn set_farm_emissions(&self) {
    //     let current_week = self.get_current_week();
    //     for farm in self.voted_farms_for_week(current_week).iter() {
    //         self.farm_proxy(farm)
    //             .set_per_block_rewards_endpoint(BigUint::zero())
    //             .execute_on_dest_context();
    //     }
    // }

    #[proxy]
    fn farm_proxy(&self, to: ManagedAddress) -> farm::Proxy<Self::Api>;
}
