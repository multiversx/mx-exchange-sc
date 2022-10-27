elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Epoch;

use simple_lock::locked_token::LockedTokenAttributes;

use crate::lock_options::MAX_PENALTY_PERCENTAGE;

const MIN_EPOCHS_TO_REDUCE: Epoch = 1;
static INVALID_PERCENTAGE_ERR_MSG: &[u8] = b"Invalid percentage value";

#[elrond_wasm::module]
pub trait UnlockWithPenaltyModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
    + crate::token_merging::TokenMergingModule
    + crate::penalty::LocalPenaltyModule
    + crate::fees::FeesModule
    + utils::UtilsModule
{
    /// Sets the percentage of fees that are burned. The rest are sent to the fees collector.
    /// Value between 0 and 10_000. 0 is also accepted.
    #[only_owner]
    #[endpoint(setFeesBurnPercentage)]
    fn set_fees_burn_percentage(&self, percentage: u16) {
        require!(
            percentage as u64 <= MAX_PENALTY_PERCENTAGE,
            INVALID_PERCENTAGE_ERR_MSG
        );

        self.fees_burn_percentage().set(percentage);
    }

    #[only_owner]
    #[endpoint(setFeesCollectorAddress)]
    fn set_fees_collector_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);
        self.fees_collector_address().set(&sc_address);
    }

    /// Unlock a locked token instantly. This incures a penalty.
    /// The longer the remaining locking time, the bigger the penalty.
    #[payable("*")]
    #[endpoint(unlockEarly)]
    fn unlock_early(&self) -> EsdtTokenPayment {
        self.reduce_lock_period_common(None)
    }

    /// Reduce the locking period of a locked token. This incures a penalty.
    /// The longer the reduction, the bigger the penalty.
    /// epochs_to_reduce must be a multiple of 360 (i.e. 1 year)
    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, target_epoch_to_reduce: Epoch) -> EsdtTokenPayment {
        self.require_is_listed_lock_option(target_epoch_to_reduce);
        self.reduce_lock_period_common(Some(target_epoch_to_reduce))
    }

    fn reduce_lock_period_common(&self, opt_epochs_to_reduce: Option<Epoch>) -> EsdtTokenPayment {
        self.require_not_paused();

        let locked_token_mapper = self.locked_token();
        let payment = self.call_value().single_esdt();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);

        let target_epochs_to_reduce =
            self.resolve_opt_epochs_to_reduce(opt_epochs_to_reduce, attributes.unlock_epoch);

        let penalty_amount = self.calculate_penalty_amount(
            &payment.amount,
            target_epochs_to_reduce,
            attributes.unlock_epoch,
        );

        locked_token_mapper.nft_burn(payment.token_nonce, &(&payment.amount - &penalty_amount));

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();

        let mut energy = self.get_updated_energy_entry_for_user(&caller);

        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);

        let mut unlocked_tokens = self.unlock_tokens_unchecked(payment.clone(), &attributes);
        let unlocked_token_id = unlocked_tokens.token_identifier.clone().unwrap_esdt();
        let new_unlock_epoch = current_epoch + target_epochs_to_reduce;

        if target_epochs_to_reduce == 0u64 {
            self.send().esdt_local_mint(
                &unlocked_token_id,
                0,
                &(&unlocked_tokens.amount - &penalty_amount),
            );
        }

        if penalty_amount > 0 {
            unlocked_tokens.amount -= &penalty_amount;
            require!(
                unlocked_tokens.amount > 0,
                "No tokens remaining after penalty is applied"
            );

            self.burn_penalty(
                locked_token_mapper.get_token_id(),
                payment.token_nonce,
                &penalty_amount,
            );
        }

        let output_payment = self.lock_and_send(&caller, unlocked_tokens, new_unlock_epoch);

        energy.add_after_token_lock(&output_payment.amount, new_unlock_epoch, current_epoch);

        self.set_energy_entry(&caller, energy);

        self.to_esdt_payment(output_payment)
    }

    fn resolve_opt_epochs_to_reduce(
        &self,
        opt_epochs_to_reduce: Option<Epoch>,
        original_unlock_epoch: Epoch,
    ) -> Epoch {
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            original_unlock_epoch > current_epoch,
            "Token can be unlocked already"
        );

        let lock_epochs_remaining = original_unlock_epoch - current_epoch;
        match opt_epochs_to_reduce {
            Some(epochs_to_reduce) => {
                require!(
                    epochs_to_reduce >= MIN_EPOCHS_TO_REDUCE
                        && epochs_to_reduce <= lock_epochs_remaining,
                    "Invalid epochs to reduce"
                );
                epochs_to_reduce
            }
            None => 0u64,
        }
    }

    fn calculate_penalty_percentage_partial_unlock(
        &self,
        target_epoch_to_reduce: Epoch,
        lock_epochs_remaining: Epoch,
    ) -> u64 {
        let penalty_percentage_full_unlock_current_epoch =
            self.calculate_penalty_percentage_full_unlock(lock_epochs_remaining);
        let penalty_percentage_full_unlock_target_epoch =
            self.calculate_penalty_percentage_full_unlock(target_epoch_to_reduce);

        (penalty_percentage_full_unlock_current_epoch - penalty_percentage_full_unlock_target_epoch)
            * MAX_PENALTY_PERCENTAGE as u64
            / (MAX_PENALTY_PERCENTAGE as u64 - penalty_percentage_full_unlock_target_epoch)
    }

    /// Calculates the penalty that would be incurred if token_amount tokens
    /// were to have their locking period reduce by epochs_to_reduce.
    /// target_epoch_to_reduce is one of 0, 360, 720, 1080 (0, 1, 2, 3 years)
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(
        &self,
        token_amount: &BigUint,
        target_epoch_to_reduce: Epoch,
        current_unlock_epoch: Epoch,
    ) -> BigUint {
        self.require_is_listed_lock_option(target_epoch_to_reduce);
        let current_epoch = self.blockchain().get_block_epoch();
        let lock_epochs_remaining = current_unlock_epoch - current_epoch;
        let full_unlock = target_epoch_to_reduce == 0;

        let penalty_percentage_unlock = if full_unlock {
            self.calculate_penalty_percentage_full_unlock(lock_epochs_remaining)
        } else {
            self.calculate_penalty_percentage_partial_unlock(
                target_epoch_to_reduce,
                lock_epochs_remaining,
            )
        };

        token_amount * penalty_percentage_unlock / MAX_PENALTY_PERCENTAGE as u64
    }
}
