use weekly_rewards_splitting::ClaimProgress;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait ProgressUpdateModule:
    week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
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

    fn update_energy_and_progress_after_enter(&self, caller: &ManagedAddress) {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(caller);

        let progress_mapper = self.current_claim_progress(caller);
        let opt_progress_for_update = if !progress_mapper.is_empty() {
            Some(progress_mapper.get())
        } else {
            None 
        };
        self.update_user_energy_for_current_week(
            caller,
            current_week,
            &current_user_energy,
            opt_progress_for_update,
        );

        progress_mapper.set(&ClaimProgress {
            week: current_week,
            energy: current_user_energy,
        });
    }
}
