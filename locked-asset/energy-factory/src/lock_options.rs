multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use unwrappable::Unwrappable;

pub const EPOCHS_PER_MONTH: Epoch = 30;
pub const EPOCHS_PER_YEAR: Epoch = 12 * EPOCHS_PER_MONTH;
pub const MAX_PENALTY_PERCENTAGE: u64 = 10_000; // 100%

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, Default)]
pub struct LockOption {
    pub lock_epochs: Epoch,
    pub penalty_start_percentage: u64,
}

pub const MAX_LOCK_OPTIONS: usize = 10;
pub type AllLockOptions = ArrayVec<LockOption, MAX_LOCK_OPTIONS>;

#[multiversx_sc::module]
pub trait LockOptionsModule {
    fn get_lock_options(&self) -> AllLockOptions {
        let options = self.lock_options().get();
        require!(!options.is_empty(), "no lock options available");

        options
    }

    fn require_is_listed_lock_option(&self, lock_epochs: Epoch) {
        let lock_options = self.get_lock_options();
        for option in &lock_options {
            if option.lock_epochs == lock_epochs {
                return;
            }
        }

        sc_panic!("Invalid lock choice");
    }

    fn unlock_epoch_to_start_of_month(&self, unlock_epoch: Epoch) -> Epoch {
        let extra_days = unlock_epoch % EPOCHS_PER_MONTH;
        unlock_epoch - extra_days
    }

    fn unlock_epoch_to_start_of_month_upper_estimate(&self, unlock_epoch: Epoch) -> Epoch {
        let lower_bound_unlock = self.unlock_epoch_to_start_of_month(unlock_epoch);
        if unlock_epoch == lower_bound_unlock {
            return lower_bound_unlock;
        }
        let new_unlock_epoch = lower_bound_unlock + EPOCHS_PER_MONTH;
        let current_epoch = self.blockchain().get_block_epoch();
        if current_epoch >= new_unlock_epoch {
            return new_unlock_epoch;
        }

        let new_lock_epochs_unbounded = new_unlock_epoch - current_epoch;
        let lock_options = self.get_lock_options();
        let last_lock_option = lock_options.last().unwrap_or_panic::<Self::Api>();
        if new_lock_epochs_unbounded <= last_lock_option.lock_epochs {
            new_unlock_epoch
        } else {
            lower_bound_unlock
        }
    }

    #[storage_mapper("lockOptions")]
    fn lock_options(&self) -> SingleValueMapper<AllLockOptions>;
}
