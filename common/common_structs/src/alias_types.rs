elrond_wasm::imports!();

use crate::UnlockSchedule;

pub type Nonce = u64;
pub type Epoch = u64;
pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;
pub type UnlockPeriod<ManagedTypeApi> = UnlockSchedule<ManagedTypeApi>;
