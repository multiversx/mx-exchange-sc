multiversx_sc::imports!();

use crate::{
    errors::{INVALID_INCENTIVE_PAYMENT, INVALID_INCENTIVE_WEEK},
    events::{ClaimedIncentiveView, FarmIncentiveView},
};
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

        let mut farm_incentives = ManagedVec::new();

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
                .update(|sum| *sum += &farm_incentive);

            farm_incentives.push(FarmIncentiveView {
                farm_address,
                amount: farm_incentive,
                week,
            });
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

        self.emit_incentivize_farm_event(incentive_payment_token, farm_incentives);
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
        let mut claimed_incentives = ManagedVec::new();

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
                user_incentive.clone(),
            ));

            let farm_address_opt = self.farm_ids().get_address(farm_id);
            if farm_address_opt.is_some() {
                let farm_address = unsafe { farm_address_opt.unwrap_unchecked() };
                claimed_incentives.push(ClaimedIncentiveView {
                    farm_address,
                    amount: user_incentive,
                });
            }
        }

        if !user_payments.is_empty() {
            self.send().direct_multi(&caller, &user_payments);

            self.emit_claim_incentive_event(week, claimed_incentives);
        }
    }
}
