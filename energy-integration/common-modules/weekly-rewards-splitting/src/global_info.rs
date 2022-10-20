elrond_wasm::imports!();

use common_types::{TokenAmountPair, Week};
use energy_query::Energy;
use week_timekeeping::EPOCHS_IN_WEEK;

#[elrond_wasm::module]
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
        let last_global_update_mapper = self.last_global_update_week();
        let last_global_update_week = last_global_update_mapper.get();
        if last_global_update_week != current_week {
            if last_global_update_week > 0 {
                let total_energy_prev_week =
                    self.total_energy_for_week(last_global_update_week).get();
                let total_tokens_prev_week = self
                    .total_locked_tokens_for_week(last_global_update_week)
                    .get();

                let week_diff = current_week - last_global_update_week;
                let energy_deplete = &total_tokens_prev_week * EPOCHS_IN_WEEK * week_diff as u64;
                let energy_for_current_week = if total_energy_prev_week >= energy_deplete {
                    total_energy_prev_week - energy_deplete
                } else {
                    BigUint::zero()
                };

                self.total_energy_for_week(current_week)
                    .set(&energy_for_current_week);
                self.total_locked_tokens_for_week(current_week)
                    .set(&total_tokens_prev_week);
            }

            last_global_update_mapper.set(current_week);
        }

        let total_removed_from_buckets = self.shift_buckets_and_get_removed_token_amount();
        let bucket_pair =
            self.reallocate_bucket_after_energy_update(prev_user_energy, current_user_energy);

        let new_total_locked_tokens =
            self.total_locked_tokens_for_week(current_week)
                .update(|total_locked| {
                    *total_locked -= total_removed_from_buckets;

                    let had_prev_energy = bucket_pair.opt_prev_bucket.is_some();
                    let has_current_energy = bucket_pair.opt_current_bucket.is_some();
                    if had_prev_energy && has_current_energy {
                        // usual case of non-zero for both prev and current energy
                        *total_locked -= prev_user_energy.get_total_locked_tokens();
                        *total_locked += current_user_energy.get_total_locked_tokens();
                    } else if had_prev_energy && !has_current_energy {
                        // only decrease if previous energy > 0,
                        // otherwise, these tokens were already removed by global shifting
                        // current not added, as it's 0
                        *total_locked -= prev_user_energy.get_total_locked_tokens();
                    } else if !had_prev_energy && has_current_energy {
                        // if user had 0 energy, but now has non-zero,
                        // then we have to only add the new tokens, as the old were already deducted
                        // during the global shifting
                        *total_locked += current_user_energy.get_total_locked_tokens();
                    }
                    // for the case when user had and has no energy, we do nothing

                    (*total_locked).clone()
                });

        let new_total_energy = self
            .total_energy_for_week(current_week)
            .update(|total_energy| {
                // revert the 7 * tokens removed in global decrease step
                if user_last_active_week != current_week {
                    *total_energy += prev_user_energy.get_total_locked_tokens() * EPOCHS_IN_WEEK;
                }

                *total_energy -= prev_user_energy.get_energy_amount();
                *total_energy += current_user_energy.get_energy_amount();

                (*total_energy).clone()
            });

        self.emit_update_global_amounts_event(
            current_week,
            &new_total_locked_tokens,
            &new_total_energy,
        )
    }

    #[view(getLastGlobalUpdateWeek)]
    #[storage_mapper("lastGlobalUpdateWeek")]
    fn last_global_update_week(&self) -> SingleValueMapper<Week>;

    #[view(getTotalRewardsForWeek)]
    #[storage_mapper("totalRewardsForWeek")]
    fn total_rewards_for_week(
        &self,
        week: Week,
    ) -> SingleValueMapper<ManagedVec<TokenAmountPair<Self::Api>>>;

    #[view(getTotalEnergyForWeek)]
    #[storage_mapper("totalEnergyForWeek")]
    fn total_energy_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getTotalLockedTokensForWeek)]
    #[storage_mapper("totalLockedTokensForWeek")]
    fn total_locked_tokens_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;
}
