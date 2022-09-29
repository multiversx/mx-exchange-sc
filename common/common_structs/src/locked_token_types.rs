elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{Epoch, EpochAmountPair};

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    PartialEq,
    TypeAbi,
    NestedEncode,
    NestedDecode,
    Clone,
    Copy,
    Debug,
)]
pub struct UnlockMilestone {
    pub unlock_epoch: u64,
    pub unlock_percent: u8,
}

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    PartialEq,
    TypeAbi,
    NestedEncode,
    NestedDecode,
    Clone,
    Copy,
    Debug,
)]
pub struct UnlockMilestoneEx {
    pub unlock_epoch: u64,
    pub unlock_percent: u64,
}

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct UnlockSchedule<M: ManagedTypeApi> {
    pub unlock_milestones: ManagedVec<M, UnlockMilestone>,
}

impl<M: ManagedTypeApi> UnlockSchedule<M> {
    pub fn from(unlock_milestones: ManagedVec<M, UnlockMilestone>) -> Self {
        UnlockSchedule { unlock_milestones }
    }
}

#[derive(
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    Clone,
    ManagedVecItem,
    TypeAbi,
    PartialEq,
    Debug,
)]
pub struct UnlockScheduleEx<M: ManagedTypeApi> {
    pub unlock_milestones: ManagedVec<M, UnlockMilestoneEx>,
}

impl<M: ManagedTypeApi> UnlockScheduleEx<M> {
    pub fn get_total_percent(&self) -> u64 {
        let mut total = 0;
        for milestone in &self.unlock_milestones {
            total += milestone.unlock_percent;
        }

        total
    }
}

#[derive(
    ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone, Debug,
)]
pub struct LockedAssetTokenAttributes<M: ManagedTypeApi> {
    pub unlock_schedule: UnlockSchedule<M>,
    pub is_merged: bool,
}

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct LockedAssetTokenAttributesEx<M: ManagedTypeApi> {
    pub unlock_schedule: UnlockScheduleEx<M>,
    pub is_merged: bool,
}

impl<M: ManagedTypeApi> LockedAssetTokenAttributesEx<M> {
    pub fn get_unlock_amounts_per_milestone<const MAX_MILESTONES_IN_SCHEDULE: usize>(
        &self,
        total_amount: &BigUint<M>,
    ) -> UnlockEpochAmountPairs<M, MAX_MILESTONES_IN_SCHEDULE> {
        let mut amounts = ArrayVec::new();
        let unlock_milestones = &self.unlock_schedule.unlock_milestones;
        if unlock_milestones.is_empty() {
            return UnlockEpochAmountPairs::new(amounts);
        }

        let mut total_tokens_processed = BigUint::zero();
        let last_milestone_index = unlock_milestones.len() - 1;
        let total_percent = self.unlock_schedule.get_total_percent();
        for (i, milestone) in unlock_milestones.iter().enumerate() {
            // account for approximation errors
            let unlock_amount_at_milestone = if i < last_milestone_index {
                total_amount * milestone.unlock_percent / total_percent
            } else {
                total_amount - &total_tokens_processed
            };

            total_tokens_processed += &unlock_amount_at_milestone;
            amounts.push(EpochAmountPair {
                epoch: milestone.unlock_epoch,
                amount: unlock_amount_at_milestone,
            });
        }

        UnlockEpochAmountPairs::new(amounts)
    }

    pub fn remove_first_milestones(&mut self, amount_to_remove: usize) {
        let unlock_milestones = &mut self.unlock_schedule.unlock_milestones;
        for _ in 0..amount_to_remove {
            unlock_milestones.remove(0);
        }
    }
}

pub struct UnlockEpochAmountPairs<M: ManagedTypeApi, const MAX_MILESTONES_IN_SCHEDULE: usize> {
    pub pairs: ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>,
}

impl<M: ManagedTypeApi, const MAX_MILESTONES_IN_SCHEDULE: usize>
    UnlockEpochAmountPairs<M, MAX_MILESTONES_IN_SCHEDULE>
{
    pub fn new(pairs: ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>) -> Self {
        Self { pairs }
    }

    pub fn get_unlockable_entries(
        &self,
        current_epoch: Epoch,
    ) -> ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE> {
        let mut unlockable_entries = ArrayVec::new();
        for pair in &self.pairs {
            if pair.epoch <= current_epoch {
                unsafe {
                    unlockable_entries.push_unchecked(pair.clone());
                }
            }
        }

        unlockable_entries
    }
}
