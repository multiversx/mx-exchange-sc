elrond_wasm::imports!();

use crate::locked_asset::PERCENTAGE_TOTAL_EX;
use common_structs::{Epoch, UnlockScheduleEx};

pub type Energy<M> = BigUint<M>;

#[elrond_wasm::module]
pub trait EnergyModule {
    fn update_energy_after_lock(
        &self,
        user: &ManagedAddress,
        lock_amount: &BigUint,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) {
        self.update_energy_for_user(user);
        self.add_new_energy_entries(user, lock_amount, unlock_schedule);
    }

    fn update_energy_buckets_after_merge(&self) {
        // TODO
    }

    fn add_new_energy_entries(
        &self,
        user: &ManagedAddress,
        lock_amount: &BigUint,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        let energy_per_lock_epoch = self.energy_per_lock_epoch().get();

        let mut total_unlock = BigUint::zero();
        let mut remaining_milestones = unlock_schedule.unlock_milestones.len();
        let mut total_energy_added = BigUint::zero();

        let mut list_mapper = self.user_unlock_epochs(&user);
        let mut current_list_node = list_mapper.front();
        for milestone in &unlock_schedule.unlock_milestones {
            // account for approximation errors
            let unlock_amount_at_milestone = if remaining_milestones > 1 {
                lock_amount * milestone.unlock_percent / PERCENTAGE_TOTAL_EX
            } else {
                lock_amount - &total_unlock
            };

            while let Some(node) = &mut current_list_node {
                let unlock_epoch_in_list = node.get_value_cloned();
                if unlock_epoch_in_list > milestone.unlock_epoch {
                    break;
                }

                let next_node_id = node.get_next_node_id();
                current_list_node = list_mapper.get_node_by_id(next_node_id);
            }

            match &mut current_list_node {
                Some(list_node) => {
                    list_mapper.push_before(list_node, milestone.unlock_epoch);
                }
                None => {
                    let _ = list_mapper.push_back(milestone.unlock_epoch);
                }
            }

            let epochs_diff = milestone.unlock_epoch - current_epoch;
            let energy_added = &energy_per_lock_epoch * epochs_diff;
            total_energy_added += energy_added;

            total_unlock += unlock_amount_at_milestone;
            remaining_milestones -= 1;
        }

        self.total_locked_tokens_for_user(user)
            .update(|total_locked| *total_locked += lock_amount);
        self.current_energy_for_user(user)
            .update(|total_energy| *total_energy += total_energy_added);
    }

    #[view(getEnergyForUser)]
    fn update_and_get_energy_for_user(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        self.update_energy_for_user(user);
        self.current_energy_for_user(user).get()
    }

    fn update_energy_for_user(&self, user: &ManagedAddress) {
        let total_locked_tokens = self.total_locked_tokens_for_user(user).get();
        if total_locked_tokens == 0 {
            return;
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let last_update_mapper = self.last_energy_update_epoch(user);
        let last_update_epoch = last_update_mapper.get();
        if last_update_epoch == current_epoch {
            return;
        }

        self.remove_expired_entries(user, current_epoch, last_update_epoch);
        self.decrease_energy_for_user(user, current_epoch, last_update_epoch);
        last_update_mapper.set(current_epoch);
    }

    fn decrease_energy_for_user(
        &self,
        user: &ManagedAddress,
        current_epoch: Epoch,
        last_update_epoch: Epoch,
    ) {
        if current_epoch == last_update_epoch {
            return;
        }

        let epoch_diff = current_epoch - last_update_epoch;
        let total_locked = self.total_locked_tokens_for_user(user).get();
        let energy_decrease = total_locked * epoch_diff;

        self.current_energy_for_user(user)
            .update(|total_energy| *total_energy -= energy_decrease);
    }

    fn remove_expired_entries(
        &self,
        user: &ManagedAddress,
        current_epoch: Epoch,
        last_update_epoch: Epoch,
    ) {
        let energy_per_lock_epoch = self.energy_per_lock_epoch().get();
        let mut epochs_mapper = self.user_unlock_epochs(user);

        let mut total_tokens_removed = BigUint::zero();
        let mut total_energy_removed = BigUint::zero();
        while let Some(list_node) = epochs_mapper.front() {
            let unlock_epoch = list_node.get_value_cloned();
            if unlock_epoch > current_epoch {
                break;
            }

            let epochs_diff = last_update_epoch - unlock_epoch;
            let energy_removed = &energy_per_lock_epoch * epochs_diff;
            total_energy_removed -= energy_removed;

            let tokens_mapper = self.tokens_for_unlock_epoch(user, unlock_epoch);
            let tokens_for_entry = tokens_mapper.get();
            total_tokens_removed += tokens_for_entry;

            tokens_mapper.clear();
            epochs_mapper.remove_node(&list_node);
        }

        if total_tokens_removed > 0 {
            self.total_locked_tokens_for_user(user)
                .update(|total_locked| *total_locked -= total_tokens_removed);
            self.current_energy_for_user(user)
                .update(|total_energy| *total_energy -= total_energy_removed);
        }
    }

    #[storage_mapper("energyPerLockEpoch")]
    fn energy_per_lock_epoch(&self) -> SingleValueMapper<Energy<Self::Api>>;

    #[storage_mapper("userUnlockEpochs")]
    fn user_unlock_epochs(&self, user: &ManagedAddress) -> LinkedListMapper<Epoch>;

    #[storage_mapper("tokensForUnlockEpoch")]
    fn tokens_for_unlock_epoch(
        &self,
        user: &ManagedAddress,
        unlock_epoch: Epoch,
    ) -> SingleValueMapper<Energy<Self::Api>>;

    #[storage_mapper("currentEnergyForUser")]
    fn current_energy_for_user(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<Energy<Self::Api>>;

    #[storage_mapper("lastEnergyUpdateEpoch")]
    fn last_energy_update_epoch(&self, user: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("totalLockedTokensForUser")]
    fn total_locked_tokens_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
