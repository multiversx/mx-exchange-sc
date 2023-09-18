multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProgressUpdateModule:
    week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    fn check_claim_progress_for_merge(&self, caller: &ManagedAddress) {
        let claim_progress_mapper = self.current_claim_progress(caller);
        if claim_progress_mapper.is_empty() {
            return;
        }

        let current_week = self.get_current_week();
        let claim_progress = claim_progress_mapper.get();
        require!(
            claim_progress.week == current_week,
            "The user claim progress must be up to date."
        )
    }
}
