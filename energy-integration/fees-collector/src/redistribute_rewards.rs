use common_types::{PaymentsVec, TokenAmountPair, Week};
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RedistributeRewardsModule:
    crate::fees_accumulation::FeesAccumulationModule
    + crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + crate::external_sc_interactions::router::RouterInteractionsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
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

        let base_token_id = self.get_base_token_id();
        let locked_token_id = self.get_locked_token_id();

        let mut all_rewards = ManagedVec::new();
        all_rewards.push(TokenAmountPair::new(base_token_id.clone(), BigUint::zero()));
        all_rewards.push(TokenAmountPair::new(
            locked_token_id.clone(),
            BigUint::zero(),
        ));

        let mut all_tokens = ManagedVec::new();
        all_tokens.push(base_token_id);
        all_tokens.push(locked_token_id);

        for week in start_week..=end_week {
            self.accumulate_remaining_rewards_single_week(&mut all_rewards, &all_tokens, week);
        }

        for reward_entry in &all_rewards {
            if reward_entry.amount == 0 {
                continue;
            }

            self.accumulated_fees(current_week, &reward_entry.token)
                .update(|acc_fees| *acc_fees += reward_entry.amount);
        }
    }

    fn accumulate_remaining_rewards_single_week(
        &self,
        all_rewards: &mut ManagedVec<TokenAmountPair<Self::Api>>,
        all_tokens: &ManagedVec<TokenIdentifier>,
        week: Week,
    ) {
        let remaining_rewards = self.remaining_rewards(week).take();
        for rem_rew_entry in &remaining_rewards {
            if rem_rew_entry.amount == 0 {
                continue;
            }

            let opt_index = all_tokens.find(&rem_rew_entry.token_identifier);
            require!(opt_index.is_some(), "Invalid setup");

            let index = unsafe { opt_index.unwrap_unchecked() };
            let mut rew_entry = all_rewards.get_mut(index);
            rew_entry.amount += rem_rew_entry.amount;
        }
    }

    #[view(getRemainingRewards)]
    #[storage_mapper("remainingRewards")]
    fn remaining_rewards(&self, week: Week) -> SingleValueMapper<PaymentsVec<Self::Api>>;
}
