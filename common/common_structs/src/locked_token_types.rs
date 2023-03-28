multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{Epoch, EpochAmountPair};

pub const MAX_MILESTONES_IN_SCHEDULE: usize = 64;
pub const PERCENTAGE_TOTAL_EX: u64 = 100_000u64;
pub const PRECISION_EX_INCREASE: u64 = 1_000u64; // From 1 to 1_000;

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

    pub fn clear_unlockable_entries(&mut self, current_epoch: Epoch) {
        let mut items_to_remove = 0usize;
        for milestone in &self.unlock_milestones {
            if milestone.unlock_epoch <= current_epoch {
                items_to_remove += 1;
            }
        }

        for _ in 0..items_to_remove {
            self.unlock_milestones.remove(0);
        }
    }

    pub fn reallocate_percentages(&mut self) {
        let current_total_percentage = self.get_total_percent();
        if current_total_percentage == PERCENTAGE_TOTAL_EX {
            return;
        }

        let mut reallocated_milestones = ManagedVec::new();
        let mut new_total = 0;
        for milestone in &self.unlock_milestones {
            let new_unlock_percentage =
                milestone.unlock_percent * PERCENTAGE_TOTAL_EX / current_total_percentage;
            if new_unlock_percentage > 0 {
                new_total += new_unlock_percentage;

                reallocated_milestones.push(UnlockMilestoneEx {
                    unlock_epoch: milestone.unlock_epoch,
                    unlock_percent: new_unlock_percentage,
                });
            }
        }

        let leftover_percent = PERCENTAGE_TOTAL_EX - new_total;
        if leftover_percent > 0 {
            let last_milestone_index = reallocated_milestones.len() - 1;
            let mut last_milestone = reallocated_milestones.get(last_milestone_index);
            last_milestone.unlock_percent += leftover_percent;

            let _ = reallocated_milestones.set(last_milestone_index, &last_milestone);
        }

        self.unlock_milestones = reallocated_milestones;
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

impl<M: ManagedTypeApi> LockedAssetTokenAttributes<M> {
    pub fn migrate_to_new_attributes(&self) -> LockedAssetTokenAttributesEx<M> {
        let mut updated_unlock_milestones: ManagedVec<M, UnlockMilestoneEx> = ManagedVec::new();
        for unlock_milestone in self.unlock_schedule.unlock_milestones.into_iter() {
            let updated_milestone = UnlockMilestoneEx {
                unlock_epoch: unlock_milestone.unlock_epoch,
                unlock_percent: unlock_milestone.unlock_percent as u64 * PRECISION_EX_INCREASE,
            };
            updated_unlock_milestones.push(updated_milestone);
        }
        let updated_unlock_schedule = UnlockScheduleEx {
            unlock_milestones: updated_unlock_milestones,
        };

        LockedAssetTokenAttributesEx {
            unlock_schedule: updated_unlock_schedule,
            is_merged: self.is_merged,
        }
    }
}

impl<M: ManagedTypeApi> LockedAssetTokenAttributesEx<M> {
    pub fn get_unlock_amounts_per_epoch(
        &self,
        total_amount: &BigUint<M>,
    ) -> UnlockEpochAmountPairs<M> {
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
            unsafe {
                amounts.push_unchecked(EpochAmountPair {
                    epoch: milestone.unlock_epoch,
                    amount: unlock_amount_at_milestone,
                });
            }
        }

        UnlockEpochAmountPairs::new(amounts)
    }
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct UnlockEpochAmountPairs<M: ManagedTypeApi> {
    pub pairs: ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>,
}

impl<M: ManagedTypeApi> UnlockEpochAmountPairs<M> {
    pub fn new(pairs: ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>) -> Self {
        Self { pairs }
    }

    pub fn get_unlockable_entries(&self, current_epoch: Epoch) -> Self {
        let mut unlockable_entries = ArrayVec::new();
        for pair in &self.pairs {
            if pair.epoch <= current_epoch {
                unsafe {
                    unlockable_entries.push_unchecked(pair.clone());
                }
            }
        }

        Self {
            pairs: unlockable_entries,
        }
    }

    pub fn get_total_unlockable_amount(&self, current_epoch: Epoch) -> BigUint<M> {
        let mut total_unlockable = BigUint::zero();
        for pair in &self.pairs {
            if pair.epoch <= current_epoch {
                total_unlockable += &pair.amount;
            }
        }

        total_unlockable
    }
}
