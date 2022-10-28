elrond_wasm::imports!();

use energy_query::Energy;
use week_timekeeping::EPOCHS_IN_WEEK;

pub type BucketId = u64;

pub struct BucketPair {
    pub opt_prev_bucket: Option<BucketId>,
    pub opt_current_bucket: Option<BucketId>,
}

#[elrond_wasm::module]
pub trait WeeklyRewardsLockedTokenBucketsModule {
    fn shift_buckets_and_get_removed_token_amount(&self, nr_pos_to_shift: usize) -> BigUint {
        let first_bucket_id_mapper = self.first_bucket_id();

        let mut total_removed_tokens = BigUint::zero();
        let mut first_bucket_id = first_bucket_id_mapper.get();
        for _ in 0..nr_pos_to_shift {
            let bucket_mapper = self.locked_tokens_in_bucket(first_bucket_id);
            let tokens_in_bucket = bucket_mapper.get();
            bucket_mapper.clear();

            total_removed_tokens += tokens_in_bucket;
            first_bucket_id += 1;
        }

        first_bucket_id_mapper.set(first_bucket_id);

        total_removed_tokens
    }

    fn reallocate_bucket_after_energy_update(
        &self,
        prev_energy: &Energy<Self::Api>,
        current_energy: &Energy<Self::Api>,
    ) -> BucketPair {
        let opt_bucket_for_prev_energy = self.get_bucket_id_for_energy(prev_energy);
        if let Some(prev_bucket_id) = &opt_bucket_for_prev_energy {
            self.locked_tokens_in_bucket(*prev_bucket_id)
                .update(|total| *total -= prev_energy.get_total_locked_tokens());
        }

        let opt_bucket_for_current_energy = self.get_bucket_id_for_energy(current_energy);
        if let Some(new_bucket_id) = &opt_bucket_for_current_energy {
            self.locked_tokens_in_bucket(*new_bucket_id)
                .update(|total| *total += current_energy.get_total_locked_tokens());
        }

        BucketPair {
            opt_prev_bucket: opt_bucket_for_prev_energy,
            opt_current_bucket: opt_bucket_for_current_energy,
        }
    }

    fn get_bucket_id_for_energy(&self, energy: &Energy<Self::Api>) -> Option<BucketId> {
        let total_tokens = energy.get_total_locked_tokens();
        if total_tokens == &0 {
            return None;
        }

        let total_energy = energy.get_energy_amount();
        if total_energy == 0 {
            return None;
        }

        let epochs_to_full_expire = total_energy / total_tokens;
        let weeks_to_full_expire = epochs_to_full_expire / EPOCHS_IN_WEEK;
        let first_bucket_id = self.first_bucket_id().get();
        let bucket_id = weeks_to_full_expire + first_bucket_id;

        // first_bucket_id will be incremented once per week.
        // total buckets will be around ~200 initially
        // This should never overflow u64
        unsafe { Some(bucket_id.to_u64().unwrap_unchecked()) }
    }

    #[storage_mapper("firstBucketId")]
    fn first_bucket_id(&self) -> SingleValueMapper<BucketId>;

    #[storage_mapper("lockedTokensInBucket")]
    fn locked_tokens_in_bucket(&self, bucket_id: BucketId) -> SingleValueMapper<BigUint>;
}
