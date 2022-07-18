elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::locked_asset::PERCENTAGE_TOTAL_EX;
use common_structs::{Epoch, UnlockScheduleEx};

#[derive(TopEncode, TopDecode)]
pub struct Energy<M: ManagedTypeApi> {
    amount: BigInt<M>,
    last_update_epoch: Epoch,
    total_locked_tokens: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for Energy<M> {
    fn default() -> Self {
        Self {
            amount: BigInt::zero(),
            last_update_epoch: 0,
            total_locked_tokens: BigUint::zero(),
        }
    }
}

impl<M: ManagedTypeApi> Energy<M> {
    pub fn deplete(&mut self, current_epoch: Epoch) {
        if self.last_update_epoch == current_epoch {
            return;
        }

        if self.total_locked_tokens > 0 && self.last_update_epoch > 0 {
            let epoch_diff = current_epoch - self.last_update_epoch;
            let energy_decrease = &self.total_locked_tokens * epoch_diff;
            self.amount -= to_bigint(energy_decrease);
        }

        self.last_update_epoch = current_epoch;
    }

    pub fn add_after_token_lock(
        &mut self,
        lock_amount: &BigUint<M>,
        unlock_schedule: &UnlockScheduleEx<M>,
        current_epoch: Epoch,
    ) {
        let milestones_len = unlock_schedule.unlock_milestones.len();
        if milestones_len == 0 {
            return;
        }

        let last_milestone_index = milestones_len - 1;
        let mut total_tokens_processed = BigUint::zero();
        for (i, milestone) in unlock_schedule.unlock_milestones.iter().enumerate() {
            // account for approximation errors
            let unlock_amount_at_milestone = if i < last_milestone_index {
                lock_amount * milestone.unlock_percent / PERCENTAGE_TOTAL_EX
            } else {
                lock_amount - &total_tokens_processed
            };

            total_tokens_processed += &unlock_amount_at_milestone;

            if current_epoch >= milestone.unlock_epoch {
                continue;
            }

            let epochs_diff = milestone.unlock_epoch - current_epoch;
            let energy_added = &unlock_amount_at_milestone * epochs_diff;
            self.amount += to_bigint(energy_added);
        }

        self.total_locked_tokens += lock_amount;
    }

    pub fn refund_after_token_unlock(
        &mut self,
        unlock_amount: &BigUint<M>,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        self.total_locked_tokens -= unlock_amount;

        if unlock_epoch == current_epoch {
            return;
        }

        let epochs_diff = current_epoch - unlock_epoch;
        let extra_energy_depleted = unlock_amount * epochs_diff;
        self.amount += to_bigint(extra_energy_depleted);
    }

    pub fn into_energy_amount(self) -> BigUint<M> {
        if self.amount >= 0 {
            self.amount.magnitude()
        } else {
            BigUint::zero()
        }
    }
}

#[elrond_wasm::module]
pub trait EnergyModule:
    crate::locked_asset::LockedAssetModule
    + token_send::TokenSendModule
    + crate::attr_ex_helper::AttrExHelper
{
    #[payable("*")]
    #[endpoint(computeEnergyForOldLockedTokens)]
    fn compute_energy_for_old_locked_tokens(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            !self.did_user_update_old_tokens(&caller),
            "Already updated old tokens"
        );

        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_or_default(&caller);
        energy.deplete(current_epoch);

        let locked_token_id = self.locked_asset_token().get_token_id();
        let energy_activation_nonce = self.energy_activation_locked_token_nonce_start().get();
        let payments = self.call_value().all_esdt_transfers();
        for payment in &payments {
            require!(
                payment.token_identifier == locked_token_id,
                "Invalid payment token"
            );
            require!(
                payment.token_nonce < energy_activation_nonce,
                "Token already acknowledged"
            );

            let token_attributes =
                self.get_attributes_ex(&payment.token_identifier, payment.token_nonce);
            energy.add_after_token_lock(
                &payment.amount,
                &token_attributes.unlock_schedule,
                current_epoch,
            );
        }

        self.user_energy(&caller).set(&energy);
        self.user_updated_energy_for_old_tokens(&caller).set(true);
    }

    fn update_energy_after_lock(
        &self,
        user: &ManagedAddress,
        lock_amount: &BigUint,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_or_default(user);

        energy.deplete(current_epoch);
        energy.add_after_token_lock(lock_amount, unlock_schedule, current_epoch);

        self.user_energy(user).set(&energy);
    }

    fn update_energy_after_merge(&self) {
        // TODO
    }

    fn update_energy_after_unlock(
        &self,
        user: &ManagedAddress,
        old_locked_token_amount: &BigUint,
        old_unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_or_default(user);
        energy.deplete(current_epoch);

        for milestone in &old_unlock_schedule.unlock_milestones {
            if milestone.unlock_epoch > current_epoch {
                continue;
            }

            let unlock_amount =
                old_locked_token_amount * milestone.unlock_percent / PERCENTAGE_TOTAL_EX;
            energy.refund_after_token_unlock(&unlock_amount, milestone.unlock_epoch, current_epoch);
        }

        self.user_energy(user).set(&energy);
    }

    fn get_energy_or_default(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        let energy_mapper = self.user_energy(user);
        if !energy_mapper.is_empty() {
            energy_mapper.get()
        } else {
            Energy::default()
        }
    }

    #[view(getEnergyForUser)]
    fn get_energy_for_user_view(&self, user: ManagedAddress) -> BigUint {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_or_default(&user);

        energy.deplete(current_epoch);

        energy.into_energy_amount()
    }

    #[inline]
    fn did_user_update_old_tokens(&self, user: &ManagedAddress) -> bool {
        self.user_updated_energy_for_old_tokens(user).get()
    }

    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;

    #[storage_mapper("userUpdatedEnergyForOldTokens")]
    fn user_updated_energy_for_old_tokens(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;

    #[storage_mapper("energyActivationLockedTokenNonceStart")]
    fn energy_activation_locked_token_nonce_start(&self) -> SingleValueMapper<u64>;
}

// temporary until added to Rust framework
fn to_bigint<M: ManagedTypeApi>(biguint: BigUint<M>) -> BigInt<M> {
    BigInt::from_raw_handle(biguint.get_raw_handle())
}
