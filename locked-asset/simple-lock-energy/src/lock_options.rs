elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Epoch;

pub const EPOCHS_PER_MONTH: Epoch = 30;
pub const EPOCHS_PER_YEAR: Epoch = 360;

#[elrond_wasm::module]
pub trait LockOptionsModule {
    /// Add lock options, as a list of epochs. Options must be >= 30 epochs (1 month).
    ///
    /// For example, an option of "60" means the user can choose to lock their tokens
    /// for 60 epochs.
    ///
    /// When calling lockTokens, users may only pick one of the whitelisted lock options.
    #[only_owner]
    #[endpoint(addLockOptions)]
    fn add_lock_options(&self, lock_options: MultiValueEncoded<Epoch>) {
        require!(!lock_options.is_empty(), "No options");

        let mut options_mapper = self.lock_options();
        let mut max_added = 0;
        for option in lock_options {
            require!(option >= EPOCHS_PER_YEAR, "Invalid option");

            if option > max_added {
                max_added = option;
            }

            let _ = options_mapper.insert(option);
        }

        self.max_lock_option().update(|max| {
            if max_added > *max {
                *max = max_added;
            }
        });
    }

    #[only_owner]
    #[endpoint(removeLockOptions)]
    fn remove_lock_options(&self, lock_options: MultiValueEncoded<Epoch>) {
        let mut options_mapper = self.lock_options();
        let max_mapper = self.max_lock_option();

        let current_max = max_mapper.get();
        let mut was_max_removed = false;
        for option in lock_options {
            if option == current_max {
                was_max_removed = true;
            }

            let _ = options_mapper.swap_remove(&option);
        }

        if was_max_removed {
            let mut new_max = 0;
            for option in options_mapper.iter() {
                if option > new_max {
                    new_max = option;
                }
            }

            max_mapper.set(new_max);
        }
    }

    fn require_is_listed_lock_option(&self, lock_epochs: Epoch) {
        require!(
            self.lock_options().contains(&lock_epochs),
            "Invalid lock choice"
        );
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
    fn lock_options(&self) -> UnorderedSetMapper<Epoch>;

    #[storage_mapper("maxLockOption")]
    fn max_lock_option(&self) -> SingleValueMapper<Epoch>;
}
