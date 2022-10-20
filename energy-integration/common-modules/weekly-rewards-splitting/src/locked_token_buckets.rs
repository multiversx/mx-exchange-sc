elrond_wasm::imports!();

use common_types::Epoch;
use energy_query::Energy;

pub type BucketId = u64;
pub const EPOCHS_PER_MONTH: Epoch = 30;

pub struct BucketPair {
    pub opt_prev_bucket: Option<BucketId>,
    pub opt_current_bucket: Option<BucketId>,
}

#[elrond_wasm::module]
pub trait WeeklyRewardsLockedTokenBucketsModule {
    fn init_next_bucket_shift_epoch(&self) {
        let current_epoch = self.blockchain().get_block_epoch();
        let start_of_month = self.epoch_to_start_of_month(current_epoch);
        let next_shift_epoch = start_of_month + EPOCHS_PER_MONTH;
        self.next_bucket_shift_epoch().set(next_shift_epoch);
    }

    fn shift_buckets_and_get_removed_token_amount(&self) -> BigUint {
        let next_update_mapper = self.next_bucket_shift_epoch();
        let first_bucket_id_mapper = self.first_bucket_id();

        let mut total_removed_tokens = BigUint::zero();
        let mut next_update_epoch = next_update_mapper.get();
        let current_epoch = self.blockchain().get_block_epoch();
        if next_update_epoch > current_epoch {
            return total_removed_tokens;
        }

        let mut first_bucket_id = first_bucket_id_mapper.get();
        while next_update_epoch <= current_epoch {
            let bucket_mapper = self.locked_tokens_in_bucket(first_bucket_id);
            let tokens_in_bucket = bucket_mapper.get();
            bucket_mapper.clear();

            total_removed_tokens += tokens_in_bucket;
            first_bucket_id += 1;
            next_update_epoch += EPOCHS_PER_MONTH;
        }

        next_update_mapper.set(next_update_epoch);
        first_bucket_id_mapper.set(first_bucket_id);

        total_removed_tokens
    }

    fn epoch_to_start_of_month(&self, epoch: Epoch) -> Epoch {
        let extra_days = epoch % EPOCHS_PER_MONTH;
        epoch - extra_days
    }

    fn reallocate_bucket_after_energy_update(
        &self,
        prev_energy: &Energy<Self::Api>,
        current_energy: &Energy<Self::Api>,
    ) -> BucketPair {
        let opt_bucket_for_prev_energy = self.get_bucket_id_for_previous_energy(prev_energy);
        if let Some(prev_bucket_id) = &opt_bucket_for_prev_energy {
            self.locked_tokens_in_bucket(*prev_bucket_id)
                .update(|total| *total -= prev_energy.get_total_locked_tokens());
        }

        let opt_bucket_for_current_energy = self.get_bucket_id_for_current_energy(current_energy);
        if let Some(new_bucket_id) = &opt_bucket_for_current_energy {
            self.locked_tokens_in_bucket(*new_bucket_id)
                .update(|total| *total += current_energy.get_total_locked_tokens());
        }

        BucketPair {
            opt_prev_bucket: opt_bucket_for_prev_energy,
            opt_current_bucket: opt_bucket_for_current_energy,
        }
    }

    fn get_bucket_id_for_current_energy(&self, energy: &Energy<Self::Api>) -> Option<BucketId> {
        let total_tokens = energy.get_total_locked_tokens();
        if total_tokens == &0 {
            return None;
        }

        let total_energy = energy.get_energy_amount();
        let epochs_to_full_expire = total_energy / total_tokens;
        if epochs_to_full_expire == 0 {
            return None;
        }

        // round exact values down
        // i.e. 30, 60, etc.
        let months_to_full_expire = (epochs_to_full_expire - 1u32) / EPOCHS_PER_MONTH;
        let first_bucket_id = self.first_bucket_id().get();
        let bucket_id = months_to_full_expire + first_bucket_id;

        // For a max period of 4 years, months_to_full_expire is max 4 * 12 = 48.
        // first_bucket_id will be incremented once per month.
        // This should never overflow u64
        unsafe { Some(bucket_id.to_u64().unwrap_unchecked()) }
    }

    fn get_bucket_id_for_previous_energy(&self, energy: &Energy<Self::Api>) -> Option<BucketId> {
        let opt_current_bucket_id = self.get_bucket_id_for_current_energy(energy);
        match opt_current_bucket_id {
            Some(current_bucket_id) => {
                let last_energy_update_epoch = energy.get_last_update_epoch();
                let next_shift_epoch = self.next_bucket_shift_epoch().get();
                let previous_shift_epoch = next_shift_epoch - EPOCHS_PER_MONTH;
                if last_energy_update_epoch >= previous_shift_epoch {
                    return Some(current_bucket_id);
                }

                let epoch_diff = previous_shift_epoch - last_energy_update_epoch;
                let shifts_missed = epoch_diff.div_ceil(EPOCHS_PER_MONTH);

                let first_bucket_id = self.first_bucket_id().get();
                let bucket_diff = current_bucket_id - first_bucket_id;
                if bucket_diff >= shifts_missed {
                    Some(current_bucket_id)
                } else {
                    // was shifted out already
                    None
                }
            }
            None => None,
        }
    }

    #[storage_mapper("nextBucketShiftEpoch")]
    fn next_bucket_shift_epoch(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("firstBucketId")]
    fn first_bucket_id(&self) -> SingleValueMapper<BucketId>;

    #[storage_mapper("lockedTokensInBucket")]
    fn locked_tokens_in_bucket(&self, bucket_id: BucketId) -> SingleValueMapper<BigUint>;
}
