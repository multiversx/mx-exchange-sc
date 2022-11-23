#![no_std]

use core::ops::Deref;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;
pub type Epoch = u64;

static NOT_ENOUGH_RESULTS_ERR_MSG: &[u8] = b"Not enough results";
const FIRST_VEC_INDEX: usize = 0;

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

pub type RawResultsType<M> = MultiValueEncoded<M, ManagedBuffer<M>>;

pub struct RawResultWrapper<M: ManagedTypeApi> {
    raw_results: ManagedVec<M, ManagedBuffer<M>>,
}

impl<M: ManagedTypeApi> RawResultWrapper<M> {
    pub fn new(raw_results: RawResultsType<M>) -> Self {
        Self {
            raw_results: raw_results.into_vec_of_buffers(),
        }
    }

    pub fn trim_results_front(&mut self, size_after_trim: usize) {
        let current_len = self.raw_results.len();
        if current_len < size_after_trim {
            M::error_api_impl().signal_error(NOT_ENOUGH_RESULTS_ERR_MSG);
        }
        if current_len == size_after_trim {
            return;
        }

        let new_start_index = current_len - size_after_trim;
        let opt_new_raw_results = self.raw_results.slice(new_start_index, current_len);
        self.raw_results = opt_new_raw_results.unwrap_or_panic::<M>();
    }

    pub fn decode_next_result<T: TopDecode>(&mut self) -> T {
        if self.raw_results.is_empty() {
            M::error_api_impl().signal_error(NOT_ENOUGH_RESULTS_ERR_MSG);
        }

        let result = {
            let raw_buffer_ref = self.raw_results.get(FIRST_VEC_INDEX);
            let decode_result = T::top_decode(raw_buffer_ref.deref().clone());
            decode_result.unwrap_or_panic::<M>()
        };
        self.raw_results.remove(FIRST_VEC_INDEX);

        result
    }
}

pub static CANNOT_UNWRAP_MSG: &[u8] = b"Cannot unwrap value";

pub trait Unwrappable<T> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T;
}

impl<T> Unwrappable<T> for Option<T> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T {
        self.unwrap_or_else(|| M::error_api_impl().signal_error(CANNOT_UNWRAP_MSG))
    }
}

impl<T, E> Unwrappable<T> for Result<T, E> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T {
        self.unwrap_or_else(|_| M::error_api_impl().signal_error(CANNOT_UNWRAP_MSG))
    }
}
