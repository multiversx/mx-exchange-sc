multiversx_sc::imports!();

use common_structs::{Epoch, Percent};

use crate::lock_options::{
    AllLockOptions, LockOption, EPOCHS_PER_YEAR, MAX_LOCK_OPTIONS, MAX_PENALTY_PERCENTAGE,
};

#[multiversx_sc::module]
pub trait LockOptionsEndpointsModule: crate::lock_options::LockOptionsModule {
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

    #[view(getLockOptions)]
    fn get_lock_options_view(&self) -> AllLockOptions {
        self.lock_options().get()
    }
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
