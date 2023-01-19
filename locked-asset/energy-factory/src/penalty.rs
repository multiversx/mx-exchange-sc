multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{Epoch, Percent};
use math::linear_interpolation;

use crate::lock_options::LockOption;

#[multiversx_sc::module]
pub trait LocalPenaltyModule: crate::lock_options::LockOptionsModule {
    fn calculate_penalty_percentage_full_unlock(&self, lock_epochs_remaining: Epoch) -> Percent {
        let lock_options = self.get_lock_options();
        let last_index = lock_options.len() - 1;
        let last_lock_option = unsafe { lock_options.get_unchecked(last_index) };
        require!(
            lock_epochs_remaining <= last_lock_option.lock_epochs,
            "Invalid lock epochs"
        );

        let mut prev_option = LockOption::default();
        let mut next_option = LockOption::default();

        let first_index = 0;
        let first_lock_option = unsafe { lock_options.get_unchecked(first_index) };
        if last_index > 0 && lock_epochs_remaining > first_lock_option.lock_epochs {
            for i in first_index..last_index {
                let prev_option_temp = unsafe { lock_options.get_unchecked(i) };
                let next_option_temp = unsafe { lock_options.get_unchecked(i + 1) };
                if prev_option_temp.lock_epochs <= lock_epochs_remaining
                    && lock_epochs_remaining <= next_option_temp.lock_epochs
                {
                    prev_option = *prev_option_temp;
                    next_option = *next_option_temp;
                    break;
                }
            }
        } else {
            // previous entry remains at the default of 0 penalty for 0 epochs
            next_option = *first_lock_option;
        }

        linear_interpolation::<Self::Api, _>(
            prev_option.lock_epochs,
            next_option.lock_epochs,
            lock_epochs_remaining,
            prev_option.penalty_start_percentage,
            next_option.penalty_start_percentage,
        )
    }
}
