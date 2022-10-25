use common_structs::Epoch;

use crate::lock_options::EPOCHS_PER_YEAR;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct PenaltyPercentage {
    pub first_threshold: u64,
    pub second_threshold: u64,
    pub third_threshold: u64,
}

#[elrond_wasm::module]
pub trait LocalPenaltyModule {
    fn calculate_penalty_percentage_full_unlock(
        &self,
        lock_epochs_remaining: Epoch,
        penalty_percentage_struct: &PenaltyPercentage,
    ) -> u64 {
        let first_threshold_penalty = penalty_percentage_struct.first_threshold;
        let second_threshold_penalty = penalty_percentage_struct.second_threshold;
        let third_threshold_penalty = penalty_percentage_struct.third_threshold;

        match lock_epochs_remaining / (EPOCHS_PER_YEAR + 1u64) {
            0 => first_threshold_penalty * lock_epochs_remaining / EPOCHS_PER_YEAR,
            1 => {
                // value between 0 and 360
                let normalized_current_epoch_unlock = lock_epochs_remaining - EPOCHS_PER_YEAR;
                first_threshold_penalty
                    + (second_threshold_penalty - first_threshold_penalty)
                        * normalized_current_epoch_unlock
                        / EPOCHS_PER_YEAR
            }
            2 | 3 => {
                // value between 721 and 1440 epochs (years 3,4) normalized to 0 - 720
                let normalized_current_epoch_unlock = lock_epochs_remaining - (2 * EPOCHS_PER_YEAR);
                second_threshold_penalty
                    + (third_threshold_penalty - second_threshold_penalty)
                        * normalized_current_epoch_unlock
                        / (2 * EPOCHS_PER_YEAR)
            }
            _ => sc_panic!("Invalid unlock choice"),
        }
    }

    fn calculate_epoch_from_penalty_percentage(
        self,
        penalty_percentage: u64,
        penalty_percentage_struct: &PenaltyPercentage,
    ) -> Epoch {
        let first_threshold_penalty = penalty_percentage_struct.first_threshold as u64;
        let second_threshold_penalty = penalty_percentage_struct.second_threshold as u64;
        let third_threshold_penalty = penalty_percentage_struct.third_threshold as u64;

        require!(
            penalty_percentage < third_threshold_penalty,
            "Invalid penalty percentage!"
        );

        if penalty_percentage > second_threshold_penalty {
            // year 2-4
            2 * EPOCHS_PER_YEAR
                + (2 * EPOCHS_PER_YEAR * penalty_percentage - second_threshold_penalty)
                    / (third_threshold_penalty - second_threshold_penalty)
        } else if penalty_percentage > first_threshold_penalty {
            // year 1-2
            EPOCHS_PER_YEAR
                + (EPOCHS_PER_YEAR * penalty_percentage - first_threshold_penalty)
                    / (second_threshold_penalty - first_threshold_penalty)
        } else if penalty_percentage > 0u64 {
            // year 0-1
            EPOCHS_PER_YEAR * penalty_percentage / first_threshold_penalty
        } else {
            sc_panic!("Invalid penalty percentage!");
        }
    }

    #[view(getPenaltyPercentage)]
    #[storage_mapper("penaltyPercentage")]
    fn penalty_percentage(&self) -> SingleValueMapper<PenaltyPercentage>;
}
