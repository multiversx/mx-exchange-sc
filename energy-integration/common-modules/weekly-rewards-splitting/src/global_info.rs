elrond_wasm::imports!();

use common_types::{TokenAmountPair, Week};
use energy_query::Energy;
use week_timekeeping::EPOCHS_IN_WEEK;

#[elrond_wasm::module]
pub trait WeeklyRewardsGlobalInfo: crate::events::WeeklyRewardsSplittingEventsModule {
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

        let total_locked_tokens_mapper = self.total_locked_tokens_for_week(current_week);
        total_locked_tokens_mapper.update(|total_locked| {
            *total_locked -= prev_user_energy.get_total_locked_tokens();
            *total_locked += current_user_energy.get_total_locked_tokens();
        });

        let total_energy_mapper = self.total_energy_for_week(current_week);
        total_energy_mapper.update(|total_energy| {
            // revert the 7 * tokens removed in global decrease step
            if user_last_active_week != current_week {
                *total_energy += prev_user_energy.get_total_locked_tokens() * EPOCHS_IN_WEEK;
            }

            *total_energy -= prev_user_energy.get_energy_amount();
            *total_energy += current_user_energy.get_energy_amount();
        });

        self.emit_update_global_amounts_event(
            last_global_update_mapper.get(),
            &total_locked_tokens_mapper.get(),
            &total_energy_mapper.get(),
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
