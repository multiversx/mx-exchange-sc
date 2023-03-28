multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use energy_query::Energy;
use math::safe_sub;
use unwrappable::Unwrappable;
use week_timekeeping::EPOCHS_IN_WEEK;

pub type BucketId = u64;

pub struct BucketPair {
    pub opt_prev_bucket: Option<BucketId>,
    pub opt_current_bucket: Option<BucketId>,
}

#[derive(TopEncode, TopDecode, PartialEq, Debug)]
pub struct LockedTokensBucket<M: ManagedTypeApi> {
    pub token_amount: BigUint<M>,
    pub surplus_energy_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for LockedTokensBucket<M> {
    fn default() -> Self {
        LockedTokensBucket {
            token_amount: BigUint::zero(),
            surplus_energy_amount: BigUint::zero(),
        }
    }
}

#[multiversx_sc::module]
pub trait WeeklyRewardsLockedTokenBucketsModule {
    fn shift_buckets_and_update_tokens_energy(
        &self,
        nr_pos_to_shift: usize,
        total_tokens: &mut BigUint,
        energy_amount: &mut BigUint,
    ) {
        let first_bucket_id_mapper = self.first_bucket_id();
        let mut first_bucket_id = first_bucket_id_mapper.get();
        for _ in 0..nr_pos_to_shift {
            let bucket_mapper = self.locked_tokens_in_bucket(first_bucket_id);
            let bucket = if !bucket_mapper.is_empty() {
                bucket_mapper.take()
            } else {
                LockedTokensBucket::default()
            };

            *total_tokens -= bucket.token_amount;
            let energy_deplete = &*total_tokens * EPOCHS_IN_WEEK + bucket.surplus_energy_amount;
            *energy_amount = safe_sub((*energy_amount).clone(), energy_deplete);

            first_bucket_id += 1;
        }

        first_bucket_id_mapper.set(first_bucket_id);
    }

    fn reallocate_bucket_after_energy_update(
        &self,
        original_prev_energy: &Energy<Self::Api>,
        depleted_prev_energy: &Energy<Self::Api>,
        current_energy: &Energy<Self::Api>,
    ) -> BucketPair {
        let opt_bucket_for_prev_energy = self.get_bucket_id_for_energy(depleted_prev_energy);
        if let Some(prev_bucket_id) = &opt_bucket_for_prev_energy {
            self.init_and_update_bucket(*prev_bucket_id, |bucket| {
                bucket.token_amount -= original_prev_energy.get_total_locked_tokens();
                bucket.surplus_energy_amount -= self.get_surplus_for_energy(original_prev_energy);
            });
        }

        let opt_bucket_for_current_energy = self.get_bucket_id_for_energy(current_energy);
        if let Some(new_bucket_id) = &opt_bucket_for_current_energy {
            self.init_and_update_bucket(*new_bucket_id, |bucket| {
                bucket.token_amount += current_energy.get_total_locked_tokens();
                bucket.surplus_energy_amount += self.get_surplus_for_energy(current_energy);
            });
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
        Some(bucket_id.to_u64().unwrap_or_panic::<Self::Api>())
    }

    fn get_surplus_for_energy(&self, energy: &Energy<Self::Api>) -> BigUint {
        let token_amount = energy.get_total_locked_tokens();
        if token_amount == &0 {
            return BigUint::zero();
        }

        energy.get_energy_amount() % (token_amount * EPOCHS_IN_WEEK)
    }

    fn init_and_update_bucket<T, UpdateFn>(&self, bucket_id: BucketId, update_fn: UpdateFn) -> T
    where
        UpdateFn: Fn(&mut LockedTokensBucket<Self::Api>) -> T,
    {
        let mapper = self.locked_tokens_in_bucket(bucket_id);
        if !mapper.is_empty() {
            return mapper.update(update_fn);
        }

        let mut new_bucket = LockedTokensBucket::default();
        let result = update_fn(&mut new_bucket);
        mapper.set(&new_bucket);

        result
    }

    #[storage_mapper("firstBucketId")]
    fn first_bucket_id(&self) -> SingleValueMapper<BucketId>;

    #[storage_mapper("lockedTokensInBucket")]
    fn locked_tokens_in_bucket(
        &self,
        bucket_id: BucketId,
    ) -> SingleValueMapper<LockedTokensBucket<Self::Api>>;
}
