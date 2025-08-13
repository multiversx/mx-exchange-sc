multiversx_sc::imports!();

use common_types::Week;
use week_timekeeping::EPOCHS_IN_WEEK;

#[multiversx_sc::module]
pub trait AdditionalLockedTokensModule:
    crate::config::ConfigModule
    + crate::fees_accumulation::FeesAccumulationModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + crate::external_sc_interactions::router::RouterInteractionsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
{
    #[only_owner]
    #[endpoint(setLockedTokensPerEpoch)]
    fn set_locked_tokens_per_epoch(&self, locked_tokens_per_epoch: BigUint) {
        self.accumulate_additional_locked_tokens();
        self.locked_tokens_per_epoch().set(locked_tokens_per_epoch);
    }

    fn accumulate_additional_locked_tokens(&self) {
        let last_update_week_mapper = self.last_locked_token_add_week();
        let last_update_week = last_update_week_mapper.get();
        let current_week = self.get_current_week();
        if last_update_week == current_week {
            return;
        }

        let epochs_in_week = EPOCHS_IN_WEEK;
        let amount_per_epoch = self.locked_tokens_per_epoch().get();
        let new_tokens_amount = amount_per_epoch * epochs_in_week;

        let locked_token_id = self.get_locked_token_id();
        self.accumulated_fees(current_week - 1, &locked_token_id)
            .update(|fees| *fees += new_tokens_amount);

        last_update_week_mapper.set(current_week);
    }

    #[view(getLastLockedTokensAddWeek)]
    #[storage_mapper("lastLockedTokenAddWeek")]
    fn last_locked_token_add_week(&self) -> SingleValueMapper<Week>;

    #[view(getLockedTokensPerEpoch)]
    #[storage_mapper("lockedTokensPerEpoch")]
    fn locked_tokens_per_epoch(&self) -> SingleValueMapper<BigUint>;
}
