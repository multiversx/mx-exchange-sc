multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_types::PaymentsVec;
use week_timekeeping::Week;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

#[multiversx_sc::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    /// Pair SC will deposit the fees through this endpoint
    /// Deposits for current week are accessible starting next week
    #[payable("*")]
    #[endpoint(depositSwapFees)]
    fn deposit_swap_fees(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            self.known_contracts().contains(&caller),
            "Only known contracts can deposit"
        );

        let payment = self.call_value().single_esdt();
        require!(
            self.known_tokens().contains(&payment.token_identifier),
            "Invalid payment token"
        );

        if payment.token_nonce > 0 {
            require!(
                payment.token_identifier == self.locked_token_id().get(),
                "Invalid locked token"
            );

            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }

        let current_week = self.get_current_week();
        self.accumulated_fees(current_week, &payment.token_identifier)
            .update(|amt| *amt += &payment.amount);

        self.emit_deposit_swap_fees_event(&caller, current_week, &payment);
    }

    #[only_admin]
    #[endpoint(redistributeRewards)]
    fn redistribute_rewards(&self, start_week: Week, end_week: Week) {
        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            "Current week must be higher than the week offset"
        );
        require!(start_week <= end_week, "Invalid week numbers");
        require!(
            end_week <= current_week - collect_rewards_offset,
            "Invalid end week"
        );

        let all_tokens = self.all_tokens().get();
        let mut all_rewards = PaymentsVec::new();
        for token_id in &all_tokens {
            all_rewards.push(EsdtTokenPayment::new(token_id, 0, BigUint::zero()));
        }

        self.accumulate_remaining_rewards(&mut all_rewards, &all_tokens, start_week, end_week);

        for reward_entry in &all_rewards {
            if reward_entry.amount == 0 {
                continue;
            }

            self.accumulated_fees(current_week, &reward_entry.token_identifier)
                .update(|acc_fees| *acc_fees += reward_entry.amount);
        }
    }

    fn accumulate_remaining_rewards(
        &self,
        all_rewards: &mut PaymentsVec<Self::Api>,
        all_tokens: &ManagedVec<TokenIdentifier>,
        start_week: Week,
        end_week: Week,
    ) {
        for week in start_week..=end_week {
            self.accumulate_remaining_rewards_single_week(all_rewards, all_tokens, week);
        }
    }

    fn accumulate_remaining_rewards_single_week(
        &self,
        all_rewards: &mut PaymentsVec<Self::Api>,
        all_tokens: &ManagedVec<TokenIdentifier>,
        week: Week,
    ) {
        let remaining_rewards = self.remaining_rewards(week).take();
        for rem_rew_entry in &remaining_rewards {
            if rem_rew_entry.amount == 0 {
                continue;
            }

            let opt_index = all_tokens.find(&rem_rew_entry.token_identifier);
            if opt_index.is_none() {
                continue;
            }

            let index = unsafe { opt_index.unwrap_unchecked() };
            let mut rew_entry = all_rewards.get_mut(index);
            rew_entry.amount += rem_rew_entry.amount;
        }
    }

    fn get_and_clear_accumulated_fees(
        &self,
        week: Week,
        token: &TokenIdentifier,
    ) -> Option<BigUint> {
        let value = self.accumulated_fees(week, token).take();
        if value > 0 {
            Some(value)
        } else {
            None
        }
    }

    #[view(getAccumulatedFees)]
    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getRemainingRewards)]
    #[storage_mapper("remainingRewards")]
    fn remaining_rewards(&self, week: Week) -> SingleValueMapper<PaymentsVec<Self::Api>>;
}
