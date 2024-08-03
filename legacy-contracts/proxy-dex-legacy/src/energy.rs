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
}
