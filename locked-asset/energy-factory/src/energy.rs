multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, Debug)]
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
    #[inline]
    pub fn new(
        amount: BigInt<M>,
        last_update_epoch: Epoch,
        total_locked_tokens: BigUint<M>,
    ) -> Self {
        Energy {
            amount,
            last_update_epoch,
            total_locked_tokens,
        }
    }

    pub fn new_zero_energy(current_epoch: Epoch) -> Self {
        Self::new(BigInt::zero(), current_epoch, BigUint::zero())
    }

    fn add(&mut self, future_epoch: Epoch, current_epoch: Epoch, amount_per_epoch: &BigUint<M>) {
        if current_epoch >= future_epoch {
            return;
        }

        let epochs_diff = future_epoch - current_epoch;
        let energy_added = amount_per_epoch * epochs_diff;
        self.amount += BigInt::from(energy_added);
    }

    fn subtract(&mut self, past_epoch: Epoch, current_epoch: Epoch, amount_per_epoch: &BigUint<M>) {
        if past_epoch >= current_epoch {
            return;
        }

        let epoch_diff = current_epoch - past_epoch;
        let energy_decrease = amount_per_epoch * epoch_diff;
        self.amount -= BigInt::from(energy_decrease);
    }

    pub fn deplete(&mut self, current_epoch: Epoch) {
        if self.last_update_epoch == current_epoch {
            return;
        }

        if self.total_locked_tokens > 0 {
            self.subtract(
                self.last_update_epoch,
                current_epoch,
                &self.total_locked_tokens.clone(),
            );
        }

        self.last_update_epoch = current_epoch;
    }

    pub fn add_energy_raw(&mut self, locked_token_amount: BigUint<M>, energy_amount: BigInt<M>) {
        self.total_locked_tokens += locked_token_amount;
        self.amount += energy_amount;
    }

    pub fn remove_energy_raw(
        &mut self,
        locked_token_amount: BigUint<M>,
        energy_amount: BigUint<M>,
    ) {
        self.total_locked_tokens -= locked_token_amount;
        self.amount -= BigInt::from(energy_amount);
    }

    pub fn add_after_token_lock(
        &mut self,
        lock_amount: &BigUint<M>,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        self.add(unlock_epoch, current_epoch, lock_amount);
        self.total_locked_tokens += lock_amount;
    }

    pub fn refund_after_token_unlock(
        &mut self,
        unlock_amount: &BigUint<M>,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        self.add(current_epoch, unlock_epoch, unlock_amount);
        self.total_locked_tokens -= unlock_amount;
    }

    pub fn deplete_after_early_unlock(
        &mut self,
        unlock_amount: &BigUint<M>,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        self.subtract(current_epoch, unlock_epoch, unlock_amount);
        self.total_locked_tokens -= unlock_amount;
    }

    pub fn update_after_unlock_any(
        &mut self,
        unlock_amount: &BigUint<M>,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        if unlock_epoch < current_epoch {
            self.refund_after_token_unlock(unlock_amount, unlock_epoch, current_epoch);
        } else {
            self.deplete_after_early_unlock(unlock_amount, unlock_epoch, current_epoch);
        }
    }

    pub fn update_after_unlock_epoch_change(
        &mut self,
        token_amount: &BigUint<M>,
        old_unlock_epoch: Epoch,
        new_unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) {
        self.update_after_unlock_any(token_amount, old_unlock_epoch, current_epoch);
        self.add_after_token_lock(token_amount, new_unlock_epoch, current_epoch);
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

    #[inline]
    pub fn get_energy_amount_raw(&self) -> &BigInt<M> {
        &self.amount
    }
}

#[multiversx_sc::module]
pub trait EnergyModule: crate::events::EventsModule {
    fn update_energy<T, F: FnOnce(&mut Energy<Self::Api>) -> T>(
        &self,
        user: &ManagedAddress,
        update_fn: F,
    ) -> T {
        let mut energy = self.get_updated_energy_entry_for_user(user);
        let result = update_fn(&mut energy);
        self.set_energy_entry(user, energy);

        result
    }

    fn set_energy_entry(&self, user: &ManagedAddress, new_energy: Energy<Self::Api>) {
        let prev_energy = self.get_updated_energy_entry_for_user(user);
        self.user_energy(user).set(&new_energy);
        self.emit_energy_updated_event(user, prev_energy, new_energy);
    }

    #[view(getEnergyEntryForUser)]
    fn get_updated_energy_entry_for_user(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        let current_epoch = self.blockchain().get_block_epoch();
        let energy_mapper = self.user_energy(user);
        if !energy_mapper.is_empty() {
            let mut energy = energy_mapper.get();
            energy.deplete(current_epoch);

            energy
        } else {
            Energy::new_zero_energy(current_epoch)
        }
    }

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        let energy = self.get_updated_energy_entry_for_user(&user);

        energy.get_energy_amount()
    }

    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;
}
