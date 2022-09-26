elrond_wasm::imports!();

use crate::{LockedAssetTokenAttributesEx, UnlockSchedule};

pub type Nonce = u64;
pub type Epoch = u64;
pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;
pub type UnlockPeriod<M> = UnlockSchedule<M>;
pub type OldLockedTokenAttributes<M> = LockedAssetTokenAttributesEx<M>;
