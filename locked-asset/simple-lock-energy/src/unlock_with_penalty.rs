elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Epoch;

use simple_lock::locked_token::LockedTokenAttributes;

use crate::lock_options::MAX_PENALTY_PERCENTAGE;

static INVALID_PERCENTAGE_ERR_MSG: &[u8] = b"Invalid percentage value";
static TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG: &[u8] = b"Token can be unlocked already";

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
    /// new_lock_period must be one of the available lock options
    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, new_lock_period: Epoch) -> EsdtTokenPayment {
        self.require_is_listed_lock_option(new_lock_period);
        self.reduce_lock_period_common(Some(new_lock_period))
    }

    fn reduce_lock_period_common(&self, opt_new_lock_period: Option<Epoch>) -> EsdtTokenPayment {
        self.require_not_paused();

        let locked_token_mapper = self.locked_token();
        let payment = self.call_value().single_esdt();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            attributes.unlock_epoch > current_epoch,
            TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
        );

        let prev_lock_epochs = attributes.unlock_epoch - current_epoch;
        let new_lock_epochs = opt_new_lock_period.unwrap_or(0);
        let penalty_amount =
            self.calculate_penalty_amount(&payment.amount, prev_lock_epochs, new_lock_epochs);

        locked_token_mapper.nft_burn(payment.token_nonce, &(&payment.amount - &penalty_amount));

        let caller = self.blockchain().get_caller();
        let mut energy = self.get_updated_energy_entry_for_user(&caller);

        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);

        let mut unlocked_tokens = self.unlock_tokens_unchecked(payment.clone(), &attributes);
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

        if new_lock_epochs == 0 {
            let unlocked_token_id = unlocked_tokens.token_identifier.clone().unwrap_esdt();
            self.send()
                .esdt_local_mint(&unlocked_token_id, 0, &unlocked_tokens.amount);
        }

        let new_unlock_epoch = current_epoch + new_lock_epochs;
        let output_payment = self.lock_and_send(&caller, unlocked_tokens, new_unlock_epoch);
        energy.add_after_token_lock(&output_payment.amount, new_unlock_epoch, current_epoch);

        self.set_energy_entry(&caller, energy);

        self.to_esdt_payment(output_payment)
    }

    fn calculate_penalty_percentage_partial_unlock(
        &self,
        prev_lock_epochs_remaining: Epoch,
        new_lock_epochs_remaining: Epoch,
    ) -> u64 {
        let prev_penalty_percentage_full =
            self.calculate_penalty_percentage_full_unlock(prev_lock_epochs_remaining);
        let new_penalty_percentage =
            self.calculate_penalty_percentage_full_unlock(new_lock_epochs_remaining);

        (prev_penalty_percentage_full - new_penalty_percentage) * MAX_PENALTY_PERCENTAGE as u64
            / (MAX_PENALTY_PERCENTAGE as u64 - new_penalty_percentage)
    }

    /// Calculates the penalty that would be incurred if token_amount tokens
    /// were to have their locking period reduce to new_unlock_epoch
    /// new_unlock_epoch must be either be current epoch (i.e. full unlock)
    /// or one of the available lock options
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(
        &self,
        token_amount: &BigUint,
        prev_lock_epochs: Epoch,
        new_lock_epochs: Epoch,
    ) -> BigUint {
        require!(prev_lock_epochs > 0, TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG);
        require!(new_lock_epochs < prev_lock_epochs, "Invalid new lock epoch");

        let is_full_unlock = new_lock_epochs == 0;
        if !is_full_unlock {
            self.require_is_listed_lock_option(new_lock_epochs);
        }

        let penalty_percentage_unlock = if is_full_unlock {
            self.calculate_penalty_percentage_full_unlock(prev_lock_epochs)
        } else {
            self.calculate_penalty_percentage_partial_unlock(prev_lock_epochs, new_lock_epochs)
        };

        token_amount * penalty_percentage_unlock / MAX_PENALTY_PERCENTAGE as u64
    }
}
