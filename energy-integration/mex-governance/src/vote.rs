multiversx_sc::imports!();

use crate::{
    config::{FarmEmission, FarmVote},
    errors::{
        ALREADY_VOTED_THIS_WEEK, FARM_BLACKLISTED, FARM_NOT_WHITELISTED, INVALID_VOTE_AMOUNT,
    },
};

#[multiversx_sc::module]
pub trait VoteModule:
    crate::config::ConfigModule
    + crate::events::EventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(vote)]
    fn vote(&self, votes: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>) {
        let caller = self.blockchain().get_caller();
        let user_id = self.user_ids().get_id_or_insert(&caller);

        let current_week = self.get_current_week();
        let voting_week = current_week + 1;

        self.advance_week_if_needed(voting_week);

        let user_votes_in_week_mapper = self.user_votes_in_week(user_id, voting_week);
        require!(
            user_votes_in_week_mapper.is_empty(),
            ALREADY_VOTED_THIS_WEEK
        );

        let user_energy = self.get_energy_amount_non_zero(&caller);

        let mut total_vote_amount = BigUint::zero();
        let mut farm_votes = ManagedVec::new();
        let mut farm_votes_event = ManagedVec::new();
        for vote in votes {
            let (farm_address, amount) = vote.into_tuple();
            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);

            require!(
                !self.blacklisted_farms().contains(&farm_id),
                FARM_BLACKLISTED
            );
            require!(
                self.whitelisted_farms().contains(&farm_id),
                FARM_NOT_WHITELISTED
            );

            self.farm_votes_for_week(farm_id, voting_week)
                .update(|sum| *sum += amount.clone());
            self.voted_farms_for_week(voting_week).insert(farm_id);

            total_vote_amount += &amount;

            farm_votes.push(FarmVote {
                farm_id,
                vote_amount: amount.clone(),
            });
            farm_votes_event.push(FarmEmission {
                farm_address,
                farm_emission: amount,
            });
        }

        require!(total_vote_amount == user_energy, INVALID_VOTE_AMOUNT);

        self.total_energy_voted(voting_week)
            .update(|sum| *sum += &user_energy);
        user_votes_in_week_mapper.set(&farm_votes);

        self.emit_vote_event(voting_week, farm_votes_event);
    }

    fn advance_week_if_needed(&self, voting_week: usize) {
        let saved_week = self.voting_week().get();
        if saved_week < voting_week {
            self.voting_week().set(voting_week);

            let emission_rate = self.reference_emission_rate().get();
            self.emission_rate_for_week(voting_week).set(emission_rate);
        }
    }
}
