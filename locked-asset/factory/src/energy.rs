elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::locked_asset::{EpochAmountPair, MAX_MILESTONES_IN_SCHEDULE};
use common_structs::{Epoch, UnlockScheduleEx};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
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
    fn add(&mut self, future_epoch: Epoch, current_epoch: Epoch, amount_per_epoch: &BigUint<M>) {
        if current_epoch >= future_epoch {
            return;
        }

        let epochs_diff = future_epoch - current_epoch;
        let energy_added = amount_per_epoch * epochs_diff;
        self.amount += to_bigint(energy_added);
    }

    fn subtract(&mut self, past_epoch: Epoch, current_epoch: Epoch, amount_per_epoch: &BigUint<M>) {
        if past_epoch >= current_epoch {
            return;
        }

        let epoch_diff = current_epoch - past_epoch;
        let energy_decrease = amount_per_epoch * epoch_diff;
        self.amount -= to_bigint(energy_decrease);
    }

    pub fn deplete(&mut self, current_epoch: Epoch) {
        if self.last_update_epoch == current_epoch {
            return;
        }

        if self.total_locked_tokens > 0 && self.last_update_epoch > 0 {
            self.subtract(
                self.last_update_epoch,
                current_epoch,
                &self.total_locked_tokens.clone(),
            );
        }

        self.last_update_epoch = current_epoch;
    }

    pub fn add_after_token_lock(
        &mut self,
        lock_amount: &BigUint<M>,
        epoch_amount_pairs: &ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>,
        current_epoch: Epoch,
    ) {
        if epoch_amount_pairs.is_empty() {
            return;
        }

        for pair in epoch_amount_pairs {
            self.add(pair.epoch, current_epoch, &pair.amount);
        }

        self.total_locked_tokens += lock_amount;
    }

    pub fn refund_after_token_unlock(
        &mut self,
        unlock_amount: &BigUint<M>,
        epoch_amount_pairs: &ArrayVec<EpochAmountPair<M>, MAX_MILESTONES_IN_SCHEDULE>,
        current_epoch: Epoch,
    ) {
        if epoch_amount_pairs.is_empty() {
            return;
        }

        for pair in epoch_amount_pairs {
            self.add(current_epoch, pair.epoch, &pair.amount);
        }

        self.total_locked_tokens -= unlock_amount;
    }

    #[inline]
    pub fn get_last_update_epoch(&self) -> Epoch {
        self.last_update_epoch
    }

    #[inline]
    pub fn get_total_locked_tokens(&self) -> &BigUint<M> {
        &self.total_locked_tokens
    }

    pub fn get_energy_amount(&self) -> BigUint<M> {
        if self.amount > 0 {
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
        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);
        let energy_activation_nonce = self.energy_activation_locked_token_nonce_start().get();

        let locked_token_id = self.locked_asset_token().get_token_id();
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
            let unlock_amounts = self.get_unlock_amounts_per_milestone(
                &token_attributes.unlock_schedule.unlock_milestones,
                &payment.amount,
            );
            energy.add_after_token_lock(&payment.amount, &unlock_amounts, current_epoch);
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
        let mut energy = self.get_updated_energy_entry_for_user(user, current_epoch);

        let unlock_amounts =
            self.get_unlock_amounts_per_milestone(&unlock_schedule.unlock_milestones, lock_amount);
        energy.add_after_token_lock(lock_amount, &unlock_amounts, current_epoch);

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
        let mut energy = self.get_updated_energy_entry_for_user(user, current_epoch);

        let unlock_amounts = self.get_unlock_amounts_per_milestone(
            &old_unlock_schedule.unlock_milestones,
            old_locked_token_amount,
        );
        energy.refund_after_token_unlock(old_locked_token_amount, &unlock_amounts, current_epoch);

        self.user_energy(user).set(&energy);
    }

    fn get_updated_energy_entry_for_user(
        &self,
        user: &ManagedAddress,
        current_epoch: u64,
    ) -> Energy<Self::Api> {
        let energy_mapper = self.user_energy(user);
        let mut energy = if !energy_mapper.is_empty() {
            energy_mapper.get()
        } else {
            Energy::default()
        };
        energy.deplete(current_epoch);

        energy
    }

    #[view(getEnergyEntryForUser)]
    fn get_energy_entry_for_user_view(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        let current_epoch = self.blockchain().get_block_epoch();
        self.get_updated_energy_entry_for_user(&user, current_epoch)
    }

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        let current_epoch = self.blockchain().get_block_epoch();
        let energy = self.get_updated_energy_entry_for_user(&user, current_epoch);

        energy.get_energy_amount()
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
