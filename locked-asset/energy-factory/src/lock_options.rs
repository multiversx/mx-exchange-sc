elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Epoch, Percent};

pub const EPOCHS_PER_MONTH: Epoch = 30;
pub const EPOCHS_PER_YEAR: Epoch = 12 * EPOCHS_PER_MONTH;
pub const MAX_PENALTY_PERCENTAGE: u64 = 10_000; // 100%

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, Default)]
pub struct LockOption {
    pub lock_epochs: Epoch,
    pub penalty_start_percentage: u64,
}

const MAX_LOCK_OPTIONS: usize = 10;
pub type AllLockOptions = ArrayVec<LockOption, MAX_LOCK_OPTIONS>;

#[elrond_wasm::module]
pub trait LockOptionsModule {
    /// Add lock options, as pairs of epochs and penalty percentages.
    /// lock epochs must be >= 360 epochs (1 year),
    /// percentages must be between 0 and 10_000
    /// Additionally, percentages must increase as lock period increases.
    ///
    /// For example, an option pair of "360, 100" means the user can choose to lock their tokens
    /// for 360 epochs, and if they were to unlock the immediately,
    /// they would incur a penalty of 1%.
    ///
    /// When calling lockTokens, or reducing lock periods,
    /// users may only pick one of the whitelisted lock options.
    #[only_owner]
    #[endpoint(addLockOptions)]
    fn add_lock_options(&self, new_lock_options: MultiValueEncoded<MultiValue2<Epoch, Percent>>) {
        self.lock_options().update(|options| {
            let new_total_options = options.len() + new_lock_options.len();
            require!(
                new_total_options <= MAX_LOCK_OPTIONS,
                "Too many lock options"
            );

            for pair in new_lock_options {
                let (lock_epochs, penalty_start_percentage) = pair.into_tuple();
                require!(
                    lock_epochs >= EPOCHS_PER_YEAR
                        && penalty_start_percentage <= MAX_PENALTY_PERCENTAGE,
                    "Invalid option"
                );

                unsafe {
                    options.push_unchecked(LockOption {
                        lock_epochs,
                        penalty_start_percentage,
                    });
                }
            }

            sort_lock_options(options);
            require_no_duplicate_lock_epoch_options::<Self::Api>(options);
            require_valid_percentages::<Self::Api>(options);
        });
    }

    #[only_owner]
    #[endpoint(removeLockOptions)]
    fn remove_lock_options(&self, options_to_remove: MultiValueEncoded<Epoch>) {
        self.lock_options().update(|options| {
            require!(
                options_to_remove.len() <= options.len(),
                "Trying to remove too many options"
            );

            let mut options_to_remove_vec = ArrayVec::<_, MAX_LOCK_OPTIONS>::new();
            for to_remove in options_to_remove {
                unsafe {
                    options_to_remove_vec.push_unchecked(to_remove);
                }
            }

            options.retain(|elem| !options_to_remove_vec.contains(&elem.lock_epochs));
        });
    }

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
        lower_bound_unlock + EPOCHS_PER_MONTH
    }

    #[view(getLockOptions)]
    #[storage_mapper("lockOptions")]
    fn lock_options(&self) -> SingleValueMapper<AllLockOptions>;
}

fn sort_lock_options(lock_options: &mut AllLockOptions) {
    lock_options.sort_unstable_by(|first, second| first.lock_epochs.cmp(&second.lock_epochs));
}

fn require_no_duplicate_lock_epoch_options<M: ManagedTypeApi>(lock_options: &AllLockOptions) {
    let end_index = lock_options.len() - 1;
    for i in 0..end_index {
        let current_element = unsafe { lock_options.get_unchecked(i) };
        let next_element = unsafe { lock_options.get_unchecked(i + 1) };
        if current_element.lock_epochs == next_element.lock_epochs {
            M::error_api_impl().signal_error(b"Duplicate lock options");
        }
    }
}

fn require_valid_percentages<M: ManagedTypeApi>(lock_options: &AllLockOptions) {
    let end_index = lock_options.len() - 1;
    for i in 0..end_index {
        let current_element = unsafe { lock_options.get_unchecked(i) };
        let next_element = unsafe { lock_options.get_unchecked(i + 1) };
        if current_element.penalty_start_percentage >= next_element.penalty_start_percentage {
            M::error_api_impl().signal_error(b"Invalid lock option percentages");
        }
    }
}
