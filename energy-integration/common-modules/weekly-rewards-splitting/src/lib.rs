#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod ongoing_operation;
pub mod events;

use common_types::{PaymentsVec, TokenAmountPair, TokenAmountPairsVec};
use energy_query::Energy;
use ongoing_operation::{CONTINUE_OP, DEFAULT_MIN_GAS_TO_SAVE_PROGRESS, STOP_OP};
use week_timekeeping::{Week, EPOCHS_IN_WEEK};

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct ClaimProgress<M: ManagedTypeApi> {
    pub energy: Energy<M>,
    pub week: Week,
}

impl<M: ManagedTypeApi> ClaimProgress<M> {
    pub fn advance_week(&mut self, opt_user_updated_energy: Option<Energy<M>>) {
        match opt_user_updated_energy {
            Some(user_updated_energy) => {
                self.energy = user_updated_energy;
            }
            None => {
                let next_week_epoch = self.energy.get_last_update_epoch() + EPOCHS_IN_WEEK;
                self.energy.deplete(next_week_epoch);
            }
        }

        self.week += 1;
    }
}

#[elrond_wasm::module]
pub trait WeeklyRewardsSplittingModule:
    energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + ongoing_operation::OngoingOperationModule
    + events::WeeklyRewardsSplittingEventsModule
{
    fn claim_multi<CollectRewardsFn: Fn(&Self, Week) -> TokenAmountPairsVec<Self::Api> + Copy>(
        &self,
        user: &ManagedAddress,
        collect_rewards_fn: CollectRewardsFn,
    ) -> PaymentsVec<Self::Api> {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(user.clone());

        self.update_user_energy_for_current_week(user, current_week, &current_user_energy);

        let claim_progress_mapper = self.current_claim_progress(user);
        let is_new_user = claim_progress_mapper.is_empty();
        let mut claim_progress = if is_new_user {
            ClaimProgress {
                energy: current_user_energy,
                week: current_week,
            }
        } else {
            claim_progress_mapper.get()
        };

        // Gas costs will increase the more weeks are claimed,
        // as the all_rewards vec will be more expensive to serialize and return
        let mut all_rewards = ManagedVec::new();
        let total_weeks_to_claim = current_week - claim_progress.week;
        let gas_for_return_data =
            (total_weeks_to_claim as u64 + 1) * DEFAULT_MIN_GAS_TO_SAVE_PROGRESS;
        let _ = self.run_while_it_has_gas(gas_for_return_data, || {
            if claim_progress.week == current_week {
                return STOP_OP;
            }

            let rewards_for_week =
                self.claim_single(user, current_week, collect_rewards_fn, &mut claim_progress);
            if !rewards_for_week.is_empty() {
                all_rewards.append_vec(rewards_for_week);
            }

            CONTINUE_OP
        });

        claim_progress_mapper.set(&claim_progress);

        self.emit_claim_multi_event(
            user,
            claim_progress.week,
            &claim_progress.energy,
            &all_rewards,
        );

        all_rewards
    }

    fn claim_single<CollectRewardsFn: Fn(&Self, Week) -> TokenAmountPairsVec<Self::Api>>(
        &self,
        user: &ManagedAddress,
        current_week: Week,
        collect_rewards_fn: CollectRewardsFn,
        claim_progress: &mut ClaimProgress<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let total_rewards =
            self.collect_and_get_rewards_for_week(claim_progress.week, collect_rewards_fn);
        let user_rewards = self.get_user_rewards_for_week(
            claim_progress.week,
            claim_progress.energy.get_energy_amount(),
            &total_rewards,
        );

        let next_week = claim_progress.week + 1;
        let next_energy_mapper = self.user_energy_for_week(user, next_week);
        let opt_next_week_energy = if !next_energy_mapper.is_empty() {
            let saved_energy = next_energy_mapper.get();
            if next_week != current_week {
                next_energy_mapper.clear();
            }

            Some(saved_energy)
        } else {
            None
        };
        claim_progress.advance_week(opt_next_week_energy);

        user_rewards
    }

    fn collect_and_get_rewards_for_week<
        CollectRewardsFn: Fn(&Self, Week) -> TokenAmountPairsVec<Self::Api>,
    >(
        &self,
        week: Week,
        collect_rewards_fn: CollectRewardsFn,
    ) -> TokenAmountPairsVec<Self::Api> {
        let total_rewards_mapper = self.total_rewards_for_week(week);
        if total_rewards_mapper.is_empty() {
            let total_rewards = collect_rewards_fn(self, week);
            total_rewards_mapper.set(&total_rewards);

            total_rewards
        } else {
            total_rewards_mapper.get()
        }
    }

    fn get_user_rewards_for_week(
        &self,
        week: Week,
        energy_amount: BigUint,
        total_rewards: &TokenAmountPairsVec<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let mut user_rewards = ManagedVec::new();
        if energy_amount == 0 {
            return user_rewards;
        }

        let total_energy = self.total_energy_for_week(week).get();
        for weekly_reward in total_rewards {
            let reward_amount = weekly_reward.amount * &energy_amount / &total_energy;
            if reward_amount > 0 {
                user_rewards.push(EsdtTokenPayment::new(weekly_reward.token, 0, reward_amount));
            }
        }

        user_rewards
    }

    fn update_user_energy_for_current_week(
        &self,
        user: &ManagedAddress,
        current_week: Week,
        current_energy: &Energy<Self::Api>,
    ) {
        let last_active_mapper = self.last_active_week_for_user(user);
        let last_active_week = last_active_mapper.get();
        let mut prev_energy = if last_active_week > 0 {
            self.user_energy_for_week(user, last_active_week).get()
        } else {
            Energy::default()
        };

        let prev_week = current_week - 1;
        if last_active_week < prev_week && last_active_week > 0 {
            let inactive_weeks = prev_week - last_active_week;
            let deplete_end_epoch =
                prev_energy.get_last_update_epoch() + inactive_weeks as u64 * EPOCHS_IN_WEEK;
            prev_energy.deplete(deplete_end_epoch);
        }

        if last_active_week != current_week {
            last_active_mapper.set(current_week);
        }

        self.user_energy_for_week(user, current_week)
            .set(current_energy);
        self.update_global_amounts_for_current_week(
            current_week,
            last_active_week,
            &prev_energy,
            current_energy,
        );

        self.emit_update_user_energy_event(user, current_week, current_energy);
    }

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
            let prev_week = current_week - 1;
            if prev_week > 0 {
                let total_energy_prev_week = self.total_energy_for_week(prev_week).get();
                let total_tokens_prev_week = self.total_locked_tokens_for_week(prev_week).get();
                let energy_deplete = &total_tokens_prev_week * EPOCHS_IN_WEEK;
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

    #[view(getUserEnergyForWeek)]
    fn get_user_energy_for_week(
        &self,
        user: &ManagedAddress,
        week: Week,
    ) -> Energy<Self::Api> {

        if self.user_energy_for_week(user, week).is_empty() {
            return Energy::default();
        }
        self.user_energy_for_week(user, week).get()
    }

    // user info

    #[view(getCurrentClaimProgress)]
    #[storage_mapper("currentClaimProgress")]
    fn current_claim_progress(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ClaimProgress<Self::Api>>;


    #[storage_mapper("userEnergyForWeek")]
    fn user_energy_for_week(
        &self,
        user: &ManagedAddress,
        week: Week,
    ) -> SingleValueMapper<Energy<Self::Api>>;

    #[view(getLastActiveWeekForUser)]
    #[storage_mapper("lastActiveWeekForUser")]
    fn last_active_week_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<Week>;

    // global info

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
