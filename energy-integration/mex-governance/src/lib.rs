#![no_std]

multiversx_sc::imports!();

pub mod config;
pub mod errors;
pub mod events;
pub mod views;

use config::{FarmVote, FarmVoteView};
use errors::*;
use week_timekeeping::Week;

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
            farm_votes_event.push(FarmVoteView {
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

    #[payable("*")]
    #[endpoint(incentivizeFarm)]
    fn incentivize_farm(
        &self,
        farms_incentives: MultiValueEncoded<MultiValue3<ManagedAddress, BigUint, Week>>,
    ) {
        let current_week = self.get_current_week();
        let mut remaining_payment = self.call_value().single_esdt();
        let incentive_payment_token = self.incentive_token().get();
        require!(
            remaining_payment.token_identifier == incentive_payment_token,
            INVALID_INCENTIVE_PAYMENT
        );

        for farm_incentive in farms_incentives {
            let (farm_address, farm_incentive, week) = farm_incentive.into_tuple();
            require!(week > current_week, INVALID_INCENTIVE_WEEK);
            require!(
                remaining_payment.amount >= farm_incentive,
                INVALID_INCENTIVE_PAYMENT
            );
            remaining_payment.amount -= &farm_incentive;

            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);

            self.farm_incentive_for_week(farm_id, week)
                .update(|sum| *sum += farm_incentive);
        }

        if remaining_payment.amount > BigUint::zero() {
            let caller = self.blockchain().get_caller();
            self.send().direct_esdt(
                &caller,
                &remaining_payment.token_identifier,
                remaining_payment.token_nonce,
                &remaining_payment.amount,
            );
        }
    }

    #[endpoint(claimIncentive)]
    fn claim_incentive(&self, week: Week) {
        let current_week = self.get_current_week();
        require!(week <= current_week, INVALID_INCENTIVE_WEEK);

        let caller = self.blockchain().get_caller();
        let user_id = self.user_ids().get_id_non_zero(&caller);
        let incentive_token = self.incentive_token().get();

        let user_votes = self.user_votes_in_week(user_id, week).get();
        let mut user_payments = ManagedVec::new();
        for user_vote in user_votes.iter() {
            let farm_id = user_vote.farm_id;

            let total_farm_incentive = self.farm_incentive_for_week(farm_id, week).get();
            if total_farm_incentive > BigUint::zero() {
                let total_farm_vote = self.farm_votes_for_week(farm_id, week).get();
                let user_incentive =
                    &total_farm_incentive * &user_vote.vote_amount / &total_farm_vote;

                user_payments.push(EsdtTokenPayment::new(
                    incentive_token.clone(),
                    0,
                    user_incentive,
                ));
            }
        }

        if user_payments.len() > 0 {
            self.send().direct_multi(&caller, &user_payments);
        }
    }

    fn advance_week_if_needed(&self, voting_week: usize) {
        let saved_week = self.voting_week().get();
        if saved_week < voting_week {
            self.voting_week().set(voting_week);

            let emission_rate = self.reference_emission_rate().get();
            self.emission_rate_for_week(voting_week).set(emission_rate);
        }
    }

    #[endpoint(setFarmEmissions)]
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
    fn farm_proxy(&self, to: ManagedAddress) -> farm_with_locked_rewards::Proxy<Self::Api>;
}
