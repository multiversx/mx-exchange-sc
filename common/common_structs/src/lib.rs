#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;
pub type Epoch = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> TokenPair<M> {
    pub fn equals(&self, other: &TokenPair<M>) -> bool {
        self.first_token == other.first_token && self.second_token == other.second_token
    }
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

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct WrappedLpTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub lp_token_total_amount: BigUint<M>,
    pub locked_assets_invested: BigUint<M>,
    pub locked_assets_nonce: Nonce,
}

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct WrappedFarmTokenAttributes<M: ManagedTypeApi> {
    pub farm_token_id: TokenIdentifier<M>,
    pub farm_token_nonce: Nonce,
    pub farm_token_amount: BigUint<M>,
    pub farming_token_id: TokenIdentifier<M>,
    pub farming_token_nonce: Nonce,
    pub farming_token_amount: BigUint<M>,
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
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

pub type UnlockPeriod<ManagedTypeApi> = UnlockSchedule<ManagedTypeApi>;

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct UnlockSchedule<M: ManagedTypeApi> {
    pub unlock_milestones: ManagedVec<M, UnlockMilestone>,
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

impl<M: ManagedTypeApi> UnlockSchedule<M> {
    pub fn from(unlock_milestones: ManagedVec<M, UnlockMilestone>) -> Self {
        UnlockSchedule { unlock_milestones }
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
    pub fn average_unlock_epoch(&self) -> Epoch {
        let mut weight_total = 0;
        let mut weighted_sum = BigUint::<M>::zero();
        for milestone in &self.unlock_schedule.unlock_milestones {
            weighted_sum += milestone.unlock_percent * milestone.unlock_epoch;
            weight_total += milestone.unlock_percent;
        }

        let weighted_average = weighted_sum / weight_total;
        unsafe { weighted_average.to_u64().unwrap_unchecked() }
    }
}
