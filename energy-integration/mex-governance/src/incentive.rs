multiversx_sc::imports!();

use crate::errors::{INVALID_INCENTIVE_PAYMENT, INVALID_INCENTIVE_WEEK};
use week_timekeeping::Week;

#[multiversx_sc::module]
pub trait IncentiveModule:
    crate::config::ConfigModule
    + crate::events::EventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
{
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

        if remaining_payment.amount > 0 {
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
            if total_farm_incentive == 0 {
                continue;
            }
            let total_farm_vote = self.farm_votes_for_week(farm_id, week).get();
            let user_incentive = &total_farm_incentive * &user_vote.vote_amount / &total_farm_vote;

            user_payments.push(EsdtTokenPayment::new(
                incentive_token.clone(),
                0,
                user_incentive,
            ));
        }

        if user_payments.len() > 0 {
            self.send().direct_multi(&caller, &user_payments);
        }
    }
}
