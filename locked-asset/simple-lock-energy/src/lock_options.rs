elrond_wasm::imports!();

use common_structs::Epoch;

#[elrond_wasm::module]
pub trait LockOptionsModule {
    /// Add lock options, as a list of epochs.
    ///
    /// For example, an option of "5" means the user can choose to lock their tokens
    /// for 5 epochs.
    ///
    /// When calling lockTokens, users may only pick one of the whitelisted lock options.
    #[only_owner]
    #[endpoint(addLockOptions)]
    fn add_lock_options(&self, lock_options: MultiValueEncoded<Epoch>) {
        let mut mapper = self.lock_options();
        for option in lock_options {
            require!(option > 0, "Invalid option");

            let _ = mapper.insert(option);
        }
    }

    #[only_owner]
    #[endpoint(removeLockOptions)]
    fn remove_lock_options(&self, lock_options: MultiValueEncoded<Epoch>) {
        let mut mapper = self.lock_options();
        for option in lock_options {
            let _ = mapper.swap_remove(&option);
        }
    }

    fn require_is_listed_lock_option(&self, lock_epochs: Epoch) {
        require!(
            self.lock_options().contains(&lock_epochs),
            "Invalid lock choice"
        );
    }

    #[view(getLockOptions)]
    #[storage_mapper("lockOptions")]
    fn lock_options(&self) -> UnorderedSetMapper<Epoch>;
}
