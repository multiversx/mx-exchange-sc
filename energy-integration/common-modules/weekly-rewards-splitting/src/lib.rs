#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const USER_MAX_CLAIM_WEEKS: usize = 4;

pub mod base_impl;
pub mod events;
pub mod global_info;
pub mod locked_token_buckets;
pub mod update_claim_progress_energy;

use base_impl::WeeklyRewardsSplittingTraitsModule;
use codec::NestedDecodeInput;
use common_structs::{PaymentsVec, Timestamp};
use energy_query::Energy;
use week_timekeeping::{Week, EPOCHS_IN_WEEK};

#[derive(TypeAbi, TopEncode, Clone, PartialEq, Debug)]
pub struct ClaimProgress<M: ManagedTypeApi> {
    pub energy: Energy<M>,
    pub week: Week,
    pub enter_timestamp: Timestamp,
}

impl<M: ManagedTypeApi> TopDecode for ClaimProgress<M> {
    fn top_decode<I>(input: I) -> Result<Self, DecodeError>
    where
        I: codec::TopDecodeInput,
    {
        let mut input_nested = input.into_nested_buffer();
        let energy = Energy::dep_decode(&mut input_nested)?;
        let week = Week::dep_decode(&mut input_nested)?;
        let enter_timestamp = if !input_nested.is_depleted() {
            Timestamp::dep_decode(&mut input_nested)?
        } else {
            0
        };

        if !input_nested.is_depleted() {
            return Result::Err(DecodeError::INPUT_TOO_LONG);
        }

        Result::Ok(ClaimProgress {
            energy,
            week,
            enter_timestamp,
        })
    }
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

#[multiversx_sc::module]
pub trait WeeklyRewardsSplittingModule:
    energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + events::WeeklyRewardsSplittingEventsModule
    + global_info::WeeklyRewardsGlobalInfo
    + locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + update_claim_progress_energy::UpdateClaimProgressEnergyModule
{
    fn claim_multi<WRSM: WeeklyRewardsSplittingTraitsModule<WeeklyRewardsSplittingMod = Self>>(
        &self,
        wrapper: &WRSM,
        user: &ManagedAddress,
    ) -> PaymentsVec<Self::Api> {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(user);

        let claim_progress_mapper = wrapper.get_claim_progress_mapper(self, user);
        let is_new_user = claim_progress_mapper.is_empty();
        let mut claim_progress = if !is_new_user {
            claim_progress_mapper.get()
        } else {
            ClaimProgress {
                energy: current_user_energy.clone(),
                week: current_week,
                enter_timestamp: self.blockchain().get_block_timestamp(),
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

        let total_weeks_to_claim = current_week - claim_progress.week;
        if total_weeks_to_claim > USER_MAX_CLAIM_WEEKS {
            let extra_weeks = total_weeks_to_claim - USER_MAX_CLAIM_WEEKS;
            claim_progress.advance_multiple_weeks(extra_weeks);
        }

        let mut all_rewards = ManagedVec::new();
        let weeks_to_claim = core::cmp::min(total_weeks_to_claim, USER_MAX_CLAIM_WEEKS);
        for _ in 0..weeks_to_claim {
            let rewards_for_week = self.claim_single(wrapper, &mut claim_progress);
            if !rewards_for_week.is_empty() {
                all_rewards.append_vec(rewards_for_week);
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
        let user_rewards = wrapper.get_user_rewards_for_week(self, claim_progress, &total_energy);

        claim_progress.advance_week();

        user_rewards
    }

    #[view(getLastActiveWeekForUser)]
    fn get_last_active_week_for_user_view(&self, user: ManagedAddress) -> Week {
        let progress_mapper = self.current_claim_progress(&user);
        if !progress_mapper.is_empty() {
            let claim_progress = progress_mapper.get();
            claim_progress.week
        } else {
            0
        }
    }

    #[view(getUserEnergyForWeek)]
    fn get_user_energy_for_week_view(
        &self,
        user: ManagedAddress,
        week: Week,
    ) -> OptionalValue<Energy<Self::Api>> {
        let progress_mapper = self.current_claim_progress(&user);
        if progress_mapper.is_empty() {
            return OptionalValue::None;
        }

        let claim_progress = progress_mapper.get();
        if claim_progress.week == week {
            OptionalValue::Some(claim_progress.energy)
        } else {
            OptionalValue::None
        }
    }
}

#[cfg(test)]
mod tests {
    use multiversx_sc_scenario::{managed_biguint, DebugApi};

    use super::*;

    #[derive(TypeAbi, TopEncode, Clone, PartialEq, Debug)]
    pub struct OldClaimProgress<M: ManagedTypeApi> {
        pub energy: Energy<M>,
        pub week: Week,
    }

    #[test]
    fn decode_old_claim_progress_to_new_test() {
        DebugApi::dummy();

        let old_progress = OldClaimProgress {
            energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
            week: 2,
        };
        let mut old_progress_encoded = ManagedBuffer::<DebugApi>::new();
        let _ = old_progress.top_encode(&mut old_progress_encoded);

        let new_progress_decoded = ClaimProgress::top_decode(old_progress_encoded).unwrap();
        assert_eq!(
            new_progress_decoded,
            ClaimProgress {
                energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
                week: 2,
                enter_timestamp: 0,
            }
        );
    }

    #[test]
    fn encoded_decode_new_progress_test() {
        DebugApi::dummy();

        let new_progress = ClaimProgress {
            energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
            week: 2,
            enter_timestamp: 0,
        };
        let mut new_progress_encoded = ManagedBuffer::<DebugApi>::new();
        let _ = new_progress.top_encode(&mut new_progress_encoded);
        let new_progress_decoded = ClaimProgress::top_decode(new_progress_encoded).unwrap();
        assert_eq!(
            new_progress_decoded,
            ClaimProgress {
                energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
                week: 2,
                enter_timestamp: 0,
            }
        );
    }
}
