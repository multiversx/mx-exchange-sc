#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;
pub type Epoch = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi, NestedEncode, NestedDecode, Clone, Copy)]
pub struct UnlockMilestone {
    pub unlock_epoch: u64,
    pub unlock_percent: u8,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone, ManagedVecItem)]
pub struct WrappedLpTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub lp_token_total_amount: BigUint<M>,
    pub locked_assets_invested: BigUint<M>,
    pub locked_assets_nonce: Nonce,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone, ManagedVecItem)]
pub struct WrappedFarmTokenAttributes<M: ManagedTypeApi> {
    pub farm_token_id: TokenIdentifier<M>,
    pub farm_token_nonce: Nonce,
    pub farm_token_amount: BigUint<M>,
    pub farming_token_id: TokenIdentifier<M>,
    pub farming_token_nonce: Nonce,
    pub farming_token_amount: BigUint<M>,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub apr_multiplier: u8,
    pub with_locked_rewards: bool,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

/*
    The two below structs (UnlockPeriod and UnlockSchedule)
    have similar structures (both of them keep a vector of
    (epoch, unlock-percent).

    The difference between them is that Period is desided
    to be used as [(number-of-epochs-until-unlock, unlock-percent)]
    whereas Schedule is [(unlock-epoch, unlock-percent)] with unlock-epoch
    being equal with current-epoch + number-of-epochs-until-unlock.

    For example:
    If current epoch is 200 and Period is [(10, 100)] (meaning that with
    a waiting time of 10 epochs, 100% of the amount will be unlocked),
    Schedule will be [(210, 100)] (meaning that at epoch 210, 100% of the
    amount will be unlocked).
*/
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct UnlockPeriod {
    pub unlock_milestones: Vec<UnlockMilestone>,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct UnlockSchedule {
    pub unlock_milestones: Vec<UnlockMilestone>,
}

impl UnlockPeriod {
    pub fn from(unlock_milestones: Vec<UnlockMilestone>) -> Self {
        UnlockPeriod { unlock_milestones }
    }
}

impl UnlockSchedule {
    pub fn from(unlock_milestones: Vec<UnlockMilestone>) -> Self {
        UnlockSchedule { unlock_milestones }
    }
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct LockedAssetTokenAttributes {
    pub unlock_schedule: UnlockSchedule,
    pub is_merged: bool,
}
