#![no_std]
#![feature(trait_alias)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const MAX_CLAIM_PER_TX: usize = 4;

pub mod base_impl;
pub mod events;
pub mod global_info;

use base_impl::WeeklyRewardsSplittingTraitsModule;
use common_types::PaymentsVec;
use energy_query::Energy;
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
    + events::WeeklyRewardsSplittingEventsModule
    + global_info::WeeklyRewardsGlobalInfo
{
    fn claim_multi<WRSM: WeeklyRewardsSplittingTraitsModule<WeeklyRewardsSplittingMod = Self>>(
        &self,
        wrapper: &WRSM,
        user: &ManagedAddress,
    ) -> PaymentsVec<Self::Api> {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(user.clone());
        let current_energy_amount = current_user_energy.get_energy_amount();

        self.update_user_energy_for_current_week(user, current_week, &current_user_energy);

        let claim_progress_mapper = WRSM::get_claim_progress_mapper(wrapper, self, user);
        let is_new_user = claim_progress_mapper.is_empty();
        let mut claim_progress = if is_new_user {
            ClaimProgress {
                energy: current_user_energy.clone(),
                week: current_week,
            }
        } else {
            claim_progress_mapper.get()
        };

        let current_epoch = self.blockchain().get_block_epoch();
        let mut calculated_energy_for_current_epoch = claim_progress.energy.clone();
        calculated_energy_for_current_epoch.deplete(current_epoch);

        let mut all_rewards = ManagedVec::new();
        if current_energy_amount >= calculated_energy_for_current_epoch.get_energy_amount() {
            let total_weeks_to_claim = current_week - claim_progress.week;
            let weeks_to_claim = core::cmp::min(total_weeks_to_claim, MAX_CLAIM_PER_TX);
            for _ in 0..weeks_to_claim {
                let rewards_for_week =
                    self.claim_single(wrapper, user, current_week, &mut claim_progress);
                if !rewards_for_week.is_empty() {
                    all_rewards.append_vec(rewards_for_week);
                }
            }
        } else {
            // for the case when a user locks, enters the weekly rewards, and then unlocks.
            // Then, they wait for a long period, and start claiming,
            // getting rewards they shouldn't have access to.
            // In this case, they receive no rewards, and their progress is reset
            claim_progress.week = current_week;
            claim_progress.energy = current_user_energy;
        }
        claim_progress_mapper.set(&claim_progress);

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
        user: &ManagedAddress,
        current_week: Week,
        claim_progress: &mut ClaimProgress<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let total_rewards = wrapper.collect_and_get_rewards_for_week(self, claim_progress.week);
        let user_rewards = self.get_user_rewards_for_week(
            claim_progress.week,
            &claim_progress.energy.get_energy_amount(),
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

    // !!! TODO  - update user boosted rewards formula
    fn get_user_rewards_for_week(
        &self,
        week: Week,
        energy_amount: &BigUint,
        total_rewards: &PaymentsVec<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let biguint_zero = BigUint::zero();
        let mut user_rewards = ManagedVec::new();
        if energy_amount == &biguint_zero {
            return user_rewards;
        }

        let total_energy = self.total_energy_for_week(week).get();
        for weekly_reward in total_rewards {
            let reward_amount = weekly_reward.amount * energy_amount / &total_energy;
            if reward_amount > 0 {
                user_rewards.push(EsdtTokenPayment::new(
                    weekly_reward.token_identifier,
                    0,
                    reward_amount,
                ));
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

    // user info

    #[view(getCurrentClaimProgress)]
    #[storage_mapper("currentClaimProgress")]
    fn current_claim_progress(
        &self,
        user: &ManagedAddress,
        nonce: Nonce,
    ) -> SingleValueMapper<ClaimProgress<Self::Api>>;

    #[view(getUserEnergyForWeek)]
    #[storage_mapper("userEnergyForWeek")]
    fn user_energy_for_week(
        &self,
        user: &ManagedAddress,
        week: Week,
    ) -> SingleValueMapper<Energy<Self::Api>>;

    #[view(getLastActiveWeekForUser)]
    #[storage_mapper("lastActiveWeekForUser")]
    fn last_active_week_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<Week>;
}
