#![no_std]
#![feature(trait_alias)]
#![feature(int_roundings)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const USER_MAX_CLAIM_WEEKS: usize = 4;

pub mod base_impl;
pub mod events;
pub mod global_info;
pub mod locked_token_buckets;

use base_impl::WeeklyRewardsSplittingTraitsModule;
use common_types::PaymentsVec;
use energy_query::Energy;
use week_timekeeping::{Week, EPOCHS_IN_WEEK};

#[derive(TypeAbi, TopEncode, TopDecode, Clone, PartialEq, Debug)]
pub struct ClaimProgress<M: ManagedTypeApi> {
    pub energy: Energy<M>,
    pub week: Week,
}

impl<M: ManagedTypeApi> ClaimProgress<M> {
    pub fn advance_week(&mut self) {
        let next_week_epoch = self.energy.get_last_update_epoch() + EPOCHS_IN_WEEK;
        self.energy.deplete(next_week_epoch);

        self.week += 1;
    }

    pub fn advance_multiple_weeks(&mut self, nr_weeks: Week) {
        let end_epoch = self.energy.get_last_update_epoch() + EPOCHS_IN_WEEK * nr_weeks as u64;
        self.energy.deplete(end_epoch);

        self.week += nr_weeks;
    }
}

#[elrond_wasm::module]
pub trait WeeklyRewardsSplittingModule:
    energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + events::WeeklyRewardsSplittingEventsModule
    + global_info::WeeklyRewardsGlobalInfo
    + locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
{
    fn claim_multi<WRSM: WeeklyRewardsSplittingTraitsModule<WeeklyRewardsSplittingMod = Self>>(
        &self,
        wrapper: &WRSM,
        user: &ManagedAddress,
    ) -> PaymentsVec<Self::Api> {
        if self.blockchain().is_smart_contract(user) {
            return PaymentsVec::new();
        }

        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(user);
        let current_energy_amount = current_user_energy.get_energy_amount();

        let claim_progress_mapper = wrapper.get_claim_progress_mapper(self, user);
        let is_new_user = claim_progress_mapper.is_empty();
        let mut claim_progress = if !is_new_user {
            claim_progress_mapper.get()
        } else {
            ClaimProgress {
                energy: current_user_energy.clone(),
                week: current_week,
            }
        };

        let opt_progress_for_energy_update = if !is_new_user {
            Some(claim_progress.clone())
        } else {
            None
        };
        self.update_user_energy_for_current_week(
            user,
            current_week,
            &current_user_energy,
            opt_progress_for_energy_update,
        );

        let current_epoch = self.blockchain().get_block_epoch();
        let mut calculated_energy_for_current_epoch = claim_progress.energy.clone();
        calculated_energy_for_current_epoch.deplete(current_epoch);

        let mut all_rewards = ManagedVec::new();

        // for the case when a user locks, enters the weekly rewards, and then unlocks.
        // Then, they wait for a long period, and start claiming,
        // getting rewards they shouldn't have access to.
        // In this case, they receive no rewards, and their progress is reset
        if current_energy_amount >= calculated_energy_for_current_epoch.get_energy_amount() {
            let total_weeks_to_claim = current_week - claim_progress.week;
            if total_weeks_to_claim > USER_MAX_CLAIM_WEEKS {
                let extra_weeks = total_weeks_to_claim - USER_MAX_CLAIM_WEEKS;
                claim_progress.advance_multiple_weeks(extra_weeks);
            }

            let weeks_to_claim = core::cmp::min(total_weeks_to_claim, USER_MAX_CLAIM_WEEKS);
            for _ in 0..weeks_to_claim {
                let rewards_for_week = self.claim_single(wrapper, &mut claim_progress);
                if !rewards_for_week.is_empty() {
                    all_rewards.append_vec(rewards_for_week);
                }
            }
        }

        claim_progress.week = current_week;
        claim_progress.energy = current_user_energy;

        if claim_progress.energy.get_energy_amount() > 0 {
            claim_progress_mapper.set(&claim_progress);
        } else {
            claim_progress_mapper.clear();
        }

        self.emit_claim_multi_event(
            user,
            claim_progress.week,
            &claim_progress.energy,
            &all_rewards,
        );

        all_rewards
    }

    fn claim_single<WRSM: WeeklyRewardsSplittingTraitsModule<WeeklyRewardsSplittingMod = Self>>(
        &self,
        wrapper: &WRSM,
        claim_progress: &mut ClaimProgress<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let total_energy = self.total_energy_for_week(claim_progress.week).get();
        let user_rewards = wrapper.get_user_rewards_for_week(
            self,
            claim_progress.week,
            &claim_progress.energy.get_energy_amount(),
            &total_energy,
        );

        claim_progress.advance_week();

        user_rewards
    }

    fn update_user_energy_for_current_week(
        &self,
        user: &ManagedAddress,
        current_week: Week,
        current_energy: &Energy<Self::Api>,
        opt_existing_claim_progres: Option<ClaimProgress<Self::Api>>,
    ) {
        let (last_active_week, prev_energy) = match opt_existing_claim_progres {
            Some(existing_claim_progress) => {
                (existing_claim_progress.week, existing_claim_progress.energy)
            }
            None => (0, Energy::default()),
        };

        // self.user_energy_for_week(user, current_week)
        //     .set(current_energy);
        self.update_global_amounts_for_current_week(
            current_week,
            last_active_week,
            &prev_energy,
            current_energy,
        );

        self.emit_update_user_energy_event(user, current_week, current_energy);
    }

    // user info

    #[view(getCurrentClaimProgress)]
    #[storage_mapper("currentClaimProgress")]
    fn current_claim_progress(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ClaimProgress<Self::Api>>;
}
