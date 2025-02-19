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

    #[endpoint(vote)]
    fn vote(&self, votes: MultiValueEncoded<MultiValue2<AddressId, BigUint>>) {
        let caller = self.blockchain().get_caller();
        let user_id = self.user_ids().get_id_or_insert(&caller);

        let current_week = self.get_current_week();
        let voting_week = current_week + 1;

        self.advance_week_if_needed(voting_week);

        require!(
            !self.users_voted_in_week(voting_week).contains(&user_id),
            ALREADY_VOTED_THIS_WEEK
        );

        let user_energy = self.get_energy_amount_non_zero(&caller);

        let mut total_vote_amount = BigUint::zero();
        let mut farm_votes = ManagedVec::new();
        for vote in votes {
            let (farm_id, amount) = vote.into_tuple();

            require!(
                self.whitelisted_farms().contains(&farm_id),
                FARM_NOT_WHITELISTED
            );
            require!(
                !self.blacklisted_farms().contains(&farm_id),
                FARM_BLACKLISTED
            );

            self.farm_votes_for_week(farm_id, voting_week)
                .update(|sum| *sum += amount.clone());
            self.voted_farms_for_week(voting_week).insert(farm_id);

            total_vote_amount += &amount;

            farm_votes.push(FarmEmission {
                farm_id,
                farm_emission: amount,
            });
        }

        require!(total_vote_amount == user_energy, INVALID_VOTE_AMOUNT);

        self.total_energy_voted(voting_week)
            .update(|sum| *sum += &user_energy);
        self.users_voted_in_week(voting_week).add(&user_id);

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

    fn set_farm_emissions(&self) {
        let current_week = self.get_current_week();
        let emission_rate = self.emission_rate_for_week(current_week).get();
        let total_votes = self.total_energy_voted(current_week).get();

        for farm_id in self.voted_farms_for_week(current_week).iter() {
            let farm_address_opt = self.farm_ids().get_address(farm_id);
            require!(farm_address_opt.is_some(), FARM_NOT_FOUND);

            let farm_address = unsafe { farm_address_opt.unwrap_unchecked() };

            let farm_votes = self.farm_votes_for_week(farm_id, current_week).get();

            let farm_emission = &emission_rate * &farm_votes / &total_votes;
            self.farm_proxy(farm_address)
                .set_per_block_rewards_endpoint(farm_emission)
                .execute_on_dest_context::<()>();
        }
    }

    #[proxy]
    fn farm_proxy(&self, to: ManagedAddress) -> farm::Proxy<Self::Api>;
}
