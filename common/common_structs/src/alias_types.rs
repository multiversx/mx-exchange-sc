use crate::UnlockSchedule;

pub type Nonce = u64;
pub type Epoch = u64;
pub type UnlockPeriod<ManagedTypeApi> = UnlockSchedule<ManagedTypeApi>;
