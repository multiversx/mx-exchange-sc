#![no_std]

use elrond_wasm::api::Handle;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;
pub type Epoch = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
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
)]
pub struct UnlockMilestone {
    pub unlock_epoch: u64,
    pub unlock_percent: u8,
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

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
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
pub struct UnlockPeriod<M: ManagedTypeApi> {
    pub unlock_milestones: ManagedVec<M, UnlockMilestone>,
}

// `derive(ManagedVecItem)` doesn't currently work with additional generics.
// Needs to be implemented by hand.
impl<M: ManagedTypeApi> ManagedVecItem<M> for UnlockPeriod<M> {
    const PAYLOAD_SIZE: usize = 4;
    const SKIPS_RESERIALIZATION: bool = false;

    fn from_byte_reader<Reader: FnMut(&mut [u8])>(api: M, reader: Reader) -> Self {
        let handle = Handle::from_byte_reader(api.clone(), reader);
        UnlockPeriod {
            unlock_milestones: ManagedVec::from_raw_handle(api, handle),
        }
    }

    fn to_byte_writer<R, Writer: FnMut(&[u8]) -> R>(&self, writer: Writer) -> R {
        <Handle as ManagedVecItem<M>>::to_byte_writer(
            &self.unlock_milestones.get_raw_handle(),
            writer,
        )
    }
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct UnlockSchedule<M: ManagedTypeApi> {
    pub unlock_milestones: ManagedVec<M, UnlockMilestone>,
}

// `derive(ManagedVecItem)` doesn't currently work with additional generics.
// Needs to be implemented by hand.
impl<M: ManagedTypeApi> ManagedVecItem<M> for UnlockSchedule<M> {
    const PAYLOAD_SIZE: usize = 4;
    const SKIPS_RESERIALIZATION: bool = false;

    fn from_byte_reader<Reader: FnMut(&mut [u8])>(api: M, reader: Reader) -> Self {
        let handle = Handle::from_byte_reader(api.clone(), reader);
        UnlockSchedule {
            unlock_milestones: ManagedVec::from_raw_handle(api, handle),
        }
    }

    fn to_byte_writer<R, Writer: FnMut(&[u8]) -> R>(&self, writer: Writer) -> R {
        <Handle as ManagedVecItem<M>>::to_byte_writer(
            &self.unlock_milestones.get_raw_handle(),
            writer,
        )
    }
}

impl<M: ManagedTypeApi> UnlockPeriod<M> {
    pub fn from(unlock_milestones: ManagedVec<M, UnlockMilestone>) -> Self {
        UnlockPeriod { unlock_milestones }
    }
}

impl<M: ManagedTypeApi> UnlockSchedule<M> {
    pub fn from(unlock_milestones: ManagedVec<M, UnlockMilestone>) -> Self {
        UnlockSchedule { unlock_milestones }
    }
}

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct LockedAssetTokenAttributes<M: ManagedTypeApi> {
    pub unlock_schedule: UnlockSchedule<M>,
    pub is_merged: bool,
}
