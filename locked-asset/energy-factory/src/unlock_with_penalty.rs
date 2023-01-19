multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;

use simple_lock::locked_token::LockedTokenAttributes;

use crate::{energy::Energy, lock_options::MAX_PENALTY_PERCENTAGE};

pub static TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG: &[u8] = b"Token can be unlocked already";

pub struct LockReduceResult<M: ManagedTypeApi> {
    pub new_lock_epochs: u64,
    pub unlocked_tokens: EgldOrEsdtTokenPayment<M>,
    pub energy: Energy<M>,
}

#[multiversx_sc::module]
pub trait UnlockWithPenaltyModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::events::EventsModule
    + multiversx_sc_modules::pause::PauseModule
    + crate::token_merging::TokenMergingModule
    + crate::penalty::LocalPenaltyModule
    + crate::unstake::UnstakeModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
    + crate::token_whitelist::TokenWhitelistModule
{
    /// Unlock a locked token instantly. This incures a penalty.
    /// The longer the remaining locking time, the bigger the penalty.
    /// Tokens can be unlocked through another SC after the unbond period has passed.
    #[payable("*")]
    #[endpoint(unlockEarly)]
    fn unlock_early(&self) {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let reduce_result = self.reduce_lock_period_common(&caller, payment.clone(), None);

        let unlocked_tokens = self.to_esdt_payment(reduce_result.unlocked_tokens);
        self.send().esdt_local_mint(
            &unlocked_tokens.token_identifier,
            0,
            &unlocked_tokens.amount,
        );

        self.set_energy_entry(&caller, reduce_result.energy);
        self.unstake_tokens(caller, payment, unlocked_tokens);
    }

    /// Reduce the locking period of a locked token. This incures a penalty.
    /// The longer the reduction, the bigger the penalty.
    /// new_lock_period must be one of the available lock options
    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, new_lock_period: Epoch) -> EsdtTokenPayment {
        self.require_is_listed_lock_option(new_lock_period);

        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let reduce_result =
            self.reduce_lock_period_common(&caller, payment.clone(), Some(new_lock_period));

        let current_epoch = self.blockchain().get_block_epoch();
        let new_unlock_epoch = current_epoch + reduce_result.new_lock_epochs;

        let unlocked_tokens = reduce_result.unlocked_tokens;
        let penalty_amount = &payment.amount - &unlocked_tokens.amount;
        let new_locked_tokens = self.lock_tokens(unlocked_tokens, new_unlock_epoch);

        let amount_to_burn = &payment.amount - &penalty_amount;
        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &amount_to_burn,
        );
        if penalty_amount > 0 {
            let fees = EsdtTokenPayment::new(
                payment.token_identifier,
                payment.token_nonce,
                penalty_amount,
            );
            self.send_fees_to_unstake_sc(fees);
        }

        let mut energy = reduce_result.energy;
        energy.add_after_token_lock(&new_locked_tokens.amount, new_unlock_epoch, current_epoch);
        self.set_energy_entry(&caller, energy);

        self.send().direct(
            &caller,
            &new_locked_tokens.token_identifier,
            new_locked_tokens.token_nonce,
            &new_locked_tokens.amount,
        );

        self.to_esdt_payment(new_locked_tokens)
    }

    fn reduce_lock_period_common(
        &self,
        caller: &ManagedAddress,
        payment: EsdtTokenPayment,
        opt_new_lock_period: Option<Epoch>,
    ) -> LockReduceResult<Self::Api> {
        self.require_not_paused();

        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            attributes.unlock_epoch > current_epoch,
            TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
        );

        let new_lock_epochs = match opt_new_lock_period {
            Some(lock_epochs) => {
                let tentative_new_unlock_epoch = current_epoch + lock_epochs;
                let start_of_month_epoch =
                    self.unlock_epoch_to_start_of_month(tentative_new_unlock_epoch);
                let epochs_diff_from_month_start =
                    tentative_new_unlock_epoch - start_of_month_epoch;

                lock_epochs - epochs_diff_from_month_start
            }
            None => 0,
        };

        let prev_lock_epochs = attributes.unlock_epoch - current_epoch;
        require!(new_lock_epochs < prev_lock_epochs, "Invalid reduce choice");

        let mut energy = self.get_updated_energy_entry_for_user(caller);
        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);

        let penalty_amount =
            self.calculate_penalty_amount(&payment.amount, prev_lock_epochs, new_lock_epochs);
        let mut unlocked_tokens = self.unlock_tokens_unchecked(payment, &attributes);
        require!(
            unlocked_tokens.amount > penalty_amount,
            "No tokens remaining after penalty is applied"
        );
        unlocked_tokens.amount -= penalty_amount;

        LockReduceResult {
            new_lock_epochs,
            energy,
            unlocked_tokens,
        }
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

        (prev_penalty_percentage_full - new_penalty_percentage) * MAX_PENALTY_PERCENTAGE
            / (MAX_PENALTY_PERCENTAGE - new_penalty_percentage)
    }

    /// Calculates the penalty that would be incurred if `token_amount` tokens
    /// were to have their lock epochs reduced from `prev_lock_epochs` to
    /// `new_lock_epochs`. For full unlock, `new_lock_epochs` should be 0.
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(
        &self,
        token_amount: &BigUint,
        prev_lock_epochs: Epoch,
        new_lock_epochs: Epoch,
    ) -> BigUint {
        require!(prev_lock_epochs > 0, TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG);
        require!(new_lock_epochs < prev_lock_epochs, "Invalid new lock epoch");

        let penalty_percentage_unlock = if new_lock_epochs == 0 {
            self.calculate_penalty_percentage_full_unlock(prev_lock_epochs)
        } else {
            self.calculate_penalty_percentage_partial_unlock(prev_lock_epochs, new_lock_epochs)
        };

        token_amount * penalty_percentage_unlock / MAX_PENALTY_PERCENTAGE
    }
}
