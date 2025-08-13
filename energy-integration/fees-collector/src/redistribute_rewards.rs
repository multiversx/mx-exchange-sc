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
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
{
    #[only_admin]
    #[endpoint(redistributeRewards)]
    fn redistribute_rewards(&self) {
        let current_week = self.get_current_week();
        let base_token_id = self.get_base_token_id();

        let token_amount_to_redistribute =
            self.get_token_available_amount(current_week, &base_token_id);

        if token_amount_to_redistribute == 0 {
            return;
        }

        self.accumulated_fees(current_week, &base_token_id)
            .update(|acc_fees| *acc_fees += token_amount_to_redistribute);
    }
}
