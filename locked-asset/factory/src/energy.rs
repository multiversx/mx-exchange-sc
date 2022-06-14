elrond_wasm::imports!();

use crate::locked_asset::PERCENTAGE_TOTAL_EX;
use common_structs::{Epoch, UnlockScheduleEx};

pub type EnergyBucketId = u64;
pub const INVALID_ENERGY_BUCKET_ID: EnergyBucketId = 0;
pub const FIRST_ENERGY_BUCKET_ID: EnergyBucketId = 1;

#[elrond_wasm::module]
pub trait EnergyModule {
    #[view(getEnergyForUser)]
    fn get_energy_for_user(&self, user: ManagedAddress) -> BigUint {
        let bucket_duration = self.bucket_duration_epochs().get();
        let current_epoch = self.blockchain().get_block_epoch();

        let first_id = self.update_and_get_first_bucket_id(current_epoch, bucket_duration);
        let last_id = self.last_bucket_id_for_user(&user).get();
        let mut total_energy = BigUint::zero();
        if first_id > last_id {
            return total_energy;
        }

        let energy_per_bucket = self.energy_per_bucket().get();
        for bucket_id in first_id..=last_id {
            let tokens_in_bucket = self.user_buckets(&user, bucket_id).get();
            if tokens_in_bucket == 0 {
                continue;
            }

            let factor = bucket_id - first_id + 1;
            let energy_in_bucket = tokens_in_bucket * &energy_per_bucket * factor;
            total_energy += energy_in_bucket;
        }

        total_energy
    }

    fn update_energy_buckets_after_lock(
        &self,
        user: &ManagedAddress,
        lock_amount: &BigUint,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) {
        let bucket_duration = self.bucket_duration_epochs().get();
        let current_epoch = self.blockchain().get_block_epoch();
        let first_bucket_id = self.update_and_get_first_bucket_id(current_epoch, bucket_duration);
        let mut max_bucket_id = 0;

        for milestone in &unlock_schedule.unlock_milestones {
            let bucket_id = self.get_bucket_id_for_unlock_epoch(
                first_bucket_id,
                bucket_duration,
                current_epoch,
                milestone.unlock_epoch,
            );
            if bucket_id == INVALID_ENERGY_BUCKET_ID {
                continue;
            }

            if bucket_id > max_bucket_id {
                max_bucket_id = bucket_id;
            }

            let unlock_amount = lock_amount * milestone.unlock_percent / PERCENTAGE_TOTAL_EX;
            self.user_buckets(user, bucket_id)
                .update(|amount_in_bucket| *amount_in_bucket += unlock_amount);
        }

        self.last_bucket_id_for_user(user).update(|last_id| {
            if max_bucket_id > *last_id {
                *last_id = max_bucket_id;
            }
        });
    }

    fn update_energy_buckets_after_merge(&self) {
        // TODO
    }

    fn get_bucket_id_for_unlock_epoch(
        &self,
        first_bucket_id: EnergyBucketId,
        bucket_duration: Epoch,
        current_epoch: Epoch,
        unlock_epoch: Epoch,
    ) -> EnergyBucketId {
        if current_epoch >= unlock_epoch {
            return INVALID_ENERGY_BUCKET_ID;
        }

        (unlock_epoch - current_epoch) / bucket_duration + first_bucket_id
    }

    fn update_and_get_first_bucket_id(
        &self,
        current_epoch: Epoch,
        bucket_duration: Epoch,
    ) -> EnergyBucketId {
        let mut first_id = self.current_global_first_bucket_id().get();
        let last_shift_epoch = self.last_bucket_shift_epoch().get();
        let buckets_to_shift = (current_epoch - last_shift_epoch) / bucket_duration;

        if buckets_to_shift > 0 {
            first_id += buckets_to_shift;

            self.current_global_first_bucket_id().set(first_id);
            self.last_bucket_shift_epoch().set(current_epoch);
        }

        first_id
    }

    #[storage_mapper("energyPerBucket")]
    fn energy_per_bucket(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("bucketDurationEpochs")]
    fn bucket_duration_epochs(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("lastBucketShiftEpoch")]
    fn last_bucket_shift_epoch(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("userBuckets")]
    fn user_buckets(
        &self,
        user: &ManagedAddress,
        bucket_id: EnergyBucketId,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("lastBucketIdForUser")]
    fn last_bucket_id_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<EnergyBucketId>;

    #[storage_mapper("currentGlobalFirstBucketId")]
    fn current_global_first_bucket_id(&self) -> SingleValueMapper<EnergyBucketId>;
}
