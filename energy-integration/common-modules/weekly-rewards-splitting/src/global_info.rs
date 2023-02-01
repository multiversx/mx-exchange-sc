multiversx_sc::imports!();

use common_types::Week;
use energy_query::Energy;
use week_timekeeping::EPOCHS_IN_WEEK;

use crate::USER_MAX_CLAIM_WEEKS;

#[multiversx_sc::module]
pub trait WeeklyRewardsGlobalInfo:
    crate::events::WeeklyRewardsSplittingEventsModule
    + crate::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
{
    fn update_global_amounts_for_current_week(
        &self,
        current_week: Week,
        user_last_active_week: Week,
        prev_user_energy: &Energy<Self::Api>,
        current_user_energy: &Energy<Self::Api>,
    ) {
        self.perform_weekly_update(current_week);

        let mut prev_energy_for_update = prev_user_energy.clone();
        if current_week != user_last_active_week {
            let week_diff = current_week - user_last_active_week;
            let deplete_end_epoch =
                prev_energy_for_update.get_last_update_epoch() + week_diff as u64 * EPOCHS_IN_WEEK;
            prev_energy_for_update.deplete(deplete_end_epoch);
        }

        let new_total_locked_tokens = self
            .update_and_get_total_tokens_amounts_after_user_energy_update(
                current_week,
                prev_user_energy,
                &prev_energy_for_update,
                current_user_energy,
            );

        let new_total_energy = self.update_and_get_total_energy_amounts_after_user_energy_update(
            current_week,
            &prev_energy_for_update,
            current_user_energy,
        );

        self.emit_update_global_amounts_event(
            current_week,
            &new_total_locked_tokens,
            &new_total_energy,
        );
    }

    fn perform_weekly_update(&self, current_week: Week) {
        let last_global_update_mapper = self.last_global_update_week();
        let last_global_update_week = last_global_update_mapper.get();
        if last_global_update_week == current_week {
            return;
        }

        last_global_update_mapper.set(current_week);

        if last_global_update_week == 0 {
            return;
        }

        let mut total_energy = self.total_energy_for_week(last_global_update_week).get();
        let mut total_tokens = self
            .total_locked_tokens_for_week(last_global_update_week)
            .take();

        let week_diff = current_week - last_global_update_week;
        self.shift_buckets_and_update_tokens_energy(
            week_diff,
            &mut total_tokens,
            &mut total_energy,
        );

        self.total_energy_for_week(current_week).set(&total_energy);
        self.total_locked_tokens_for_week(current_week)
            .set(&total_tokens);

        // clear entries that are not accessible anymore
        // users can claim only for weeks of
        // (current_week - 1), (current_week - 2), ... (current_week - USER_MAX_CLAIM_WEEKS)
        if current_week > USER_MAX_CLAIM_WEEKS + 1 {
            let inaccessible_week = current_week - USER_MAX_CLAIM_WEEKS - 1;
            self.total_rewards_for_week(inaccessible_week).clear();
            self.total_energy_for_week(inaccessible_week).clear();
        }
    }

    fn update_and_get_total_tokens_amounts_after_user_energy_update(
        &self,
        current_week: Week,
        original_prev_user_energy: &Energy<Self::Api>,
        depleted_prev_user_energy: &Energy<Self::Api>,
        current_user_energy: &Energy<Self::Api>,
    ) -> BigUint {
        let bucket_pair = self.reallocate_bucket_after_energy_update(
            original_prev_user_energy,
            depleted_prev_user_energy,
            current_user_energy,
        );

        self.total_locked_tokens_for_week(current_week)
            .update(|total_locked| {
                let had_prev_energy = bucket_pair.opt_prev_bucket.is_some();
                let has_current_energy = bucket_pair.opt_current_bucket.is_some();
                if had_prev_energy && has_current_energy {
                    // usual case of non-zero for both prev and current energy
                    *total_locked += current_user_energy.get_total_locked_tokens();
                    *total_locked -= depleted_prev_user_energy.get_total_locked_tokens();
                } else if had_prev_energy && !has_current_energy {
                    // only decrease if previous energy > 0,
                    // otherwise, these tokens were already removed by global shifting
                    // current not added, as it's 0
                    *total_locked -= depleted_prev_user_energy.get_total_locked_tokens();
                } else if !had_prev_energy && has_current_energy {
                    // if user had 0 energy, but now has non-zero,
                    // then we have to only add the new tokens, as the old were already deducted
                    // during the global shifting
                    *total_locked += current_user_energy.get_total_locked_tokens();
                }
                // for the case when user had and has no energy, we do nothing

                (*total_locked).clone()
            })
    }

    fn update_and_get_total_energy_amounts_after_user_energy_update(
        &self,
        current_week: Week,
        prev_user_energy: &Energy<Self::Api>,
        current_user_energy: &Energy<Self::Api>,
    ) -> BigUint {
        self.total_energy_for_week(current_week)
            .update(|total_energy| {
                *total_energy -= prev_user_energy.get_energy_amount();
                *total_energy += current_user_energy.get_energy_amount();

                (*total_energy).clone()
            })
    }

    #[view(getLastGlobalUpdateWeek)]
    #[storage_mapper("lastGlobalUpdateWeek")]
    fn last_global_update_week(&self) -> SingleValueMapper<Week>;

    #[view(getTotalRewardsForWeek)]
    #[storage_mapper("totalRewardsForWeek")]
    fn total_rewards_for_week(
        &self,
        week: Week,
    ) -> SingleValueMapper<ManagedVec<EsdtTokenPayment<Self::Api>>>;

    #[view(getTotalEnergyForWeek)]
    #[storage_mapper("totalEnergyForWeek")]
    fn total_energy_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getTotalLockedTokensForWeek)]
    #[storage_mapper("totalLockedTokensForWeek")]
    fn total_locked_tokens_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;
}
