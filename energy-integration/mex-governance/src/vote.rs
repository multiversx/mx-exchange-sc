multiversx_sc::imports!();

use crate::{
    config::{FarmEmission, FarmVote, MAX_FARMS_PER_VOTE, MAX_REWARDED_FARMS},
    errors::{ALREADY_VOTED_THIS_WEEK, FARM_BLACKLISTED, INVALID_VOTE_AMOUNT},
};

#[multiversx_sc::module]
pub trait VoteModule:
    crate::config::ConfigModule
    + crate::events::EventsModule
    + crate::external_interactions::energy_factory_interactions::EnergyFactoryInteractionsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(vote)]
    fn vote(&self, votes: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>) {
        require!(
            votes.len() <= MAX_FARMS_PER_VOTE,
            "Too many farms in one vote"
        );

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

            self.check_farm_is_whitelisted(&farm_address);
            let farm_id = self.farm_ids().get_id_or_insert(&farm_address);
            require!(
                !self.blacklisted_farms().contains(&farm_id),
                FARM_BLACKLISTED
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

        self.update_top_farms(voting_week, &farm_votes_event);

        self.emit_vote_event(voting_week, farm_votes_event);
    }

    fn update_top_farms(&self, week: usize, new_votes: &ManagedVec<FarmEmission<Self::Api>>) {
        let mut top_farms = self.farm_emissions_for_week(week).get();
        let mut redistributed_votes = self.redistributed_votes_for_week(week).get();

        self.add_new_votes(week, &mut top_farms, new_votes);
        self.selection_sort_farms_desc(&mut top_farms);
        self.trim_top_farms(week, &mut top_farms, &mut redistributed_votes);

        self.farm_emissions_for_week(week).set(&top_farms);
        self.redistributed_votes_for_week(week)
            .set(&redistributed_votes);
    }

    fn add_new_votes(
        &self,
        week: usize,
        top_farms: &mut ManagedVec<FarmEmission<Self::Api>>,
        new_votes: &ManagedVec<FarmEmission<Self::Api>>,
    ) {
        for farm_vote in new_votes.iter() {
            let farm_address = farm_vote.farm_address.clone();
            let vote_amount = farm_vote.farm_emission.clone();

            let is_in_top_farms = self.top_farms_whitelist(week).contains(&farm_address);

            if is_in_top_farms {
                for i in 0..top_farms.len() {
                    if &top_farms.get(i).farm_address == &farm_address {
                        let mut farm_emission = top_farms.get(i);
                        farm_emission.farm_emission += vote_amount;
                        let _ = top_farms.set(i, &farm_emission);
                        break;
                    }
                }
            } else {
                // Add new farm to the list, even if it goes beyond max limit. Also, add to whitelist temporarily.
                // This will help simplify the sorting logic later on
                top_farms.push(FarmEmission {
                    farm_address: farm_address.clone(),
                    farm_emission: vote_amount,
                });

                self.top_farms_whitelist(week).add(&farm_address);
            }
        }
    }

    // Selection sort algorith
    fn selection_sort_farms_desc(&self, farms: &mut ManagedVec<FarmEmission<Self::Api>>) {
        let len = farms.len();

        if len <= 1 {
            return;
        }

        for i in 0..len - 1 {
            let mut max_idx = i;
            for j in i + 1..len {
                if farms.get(j).farm_emission > farms.get(max_idx).farm_emission {
                    max_idx = j;
                }
            }

            if max_idx != i {
                let max_farm = farms.get(max_idx);
                let current_farm = farms.get(i);

                let _ = farms.set(i, &max_farm);
                let _ = farms.set(max_idx, &current_farm);
            }
        }
    }

    fn trim_top_farms(
        &self,
        week: usize,
        top_farms: &mut ManagedVec<FarmEmission<Self::Api>>,
        redistributed_votes: &mut BigUint,
    ) {
        if top_farms.len() > MAX_REWARDED_FARMS {
            for i in MAX_REWARDED_FARMS..top_farms.len() {
                *redistributed_votes += &top_farms.get(i).farm_emission;

                self.top_farms_whitelist(week)
                    .remove(&top_farms.get(i).farm_address);
            }

            let top_farms_slice = top_farms.slice(0, MAX_REWARDED_FARMS).unwrap_or_default();
            *top_farms = top_farms_slice;
        }
    }

    fn advance_week_if_needed(&self, voting_week: usize) {
        let saved_week = self.voting_week().get();
        if saved_week < voting_week {
            self.voting_week().set(voting_week);

            let emission_rate = self.reference_emission_rate().get();
            self.emission_rate_for_week(voting_week).set(emission_rate);

            self.farm_emissions_for_week(voting_week)
                .set(ManagedVec::new());
            self.redistributed_votes_for_week(voting_week)
                .set(BigUint::zero());
        }
    }
}
