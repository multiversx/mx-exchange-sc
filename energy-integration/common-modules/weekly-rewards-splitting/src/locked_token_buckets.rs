elrond_wasm::imports!();

use common_types::Epoch;
use energy_query::Energy;

pub type BucketId = u64;
pub const EPOCHS_PER_MONTH: Epoch = 30;

#[elrond_wasm::module]
pub trait WeeklyRewardsLockedTokenBucketsModule {
    fn init_next_bucket_shift_epoch(&self) {
        let current_epoch = self.blockchain().get_block_epoch();
        let start_of_month = self.epoch_to_start_of_month(current_epoch);
        let next_shift_epoch = start_of_month + EPOCHS_PER_MONTH;
        self.next_bucket_shift_epoch().set(next_shift_epoch);
    }

    fn add_new_tokens_to_buckets(&self) {}

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

        let months_to_full_expire = epochs_to_full_expire / EPOCHS_PER_MONTH;
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

                // rounded up
                let shift_epochs_missed =
                    (previous_shift_epoch - last_energy_update_epoch + EPOCHS_PER_MONTH - 1)
                        / EPOCHS_PER_MONTH;
                if shift_epochs_missed >= current_bucket_id {
                    return None;
                } else {
                    // for each shift missed, it means the user is currently in one bucket to the left
                    Some(current_bucket_id - shift_epochs_missed)
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
