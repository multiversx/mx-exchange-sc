elrond_wasm::imports!();

use common_structs::Epoch;
use simple_lock::locked_token::LockedTokenAttributes;

use crate::energy::Energy;

#[elrond_wasm::module]
pub trait ExtendLockModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::old_token_nonces::OldTokenNonces
    + crate::old_token_actions::OldTokenActions
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
{
    /// Extend locking period of a previously locked token.
    /// new_lock_period must still be one of the available options.
    /// No penalty is received for performing this action.
    #[payable("*")]
    #[endpoint(extendLockingPeriod)]
    fn extend_locking_period_endpoint(&self, new_lock_period: Epoch) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_is_listed_lock_option(new_lock_period);

        let payment = self.call_value().single_esdt();
        self.locked_token()
            .require_same_token(&payment.token_identifier);

        let current_epoch = self.blockchain().get_block_epoch();
        let new_unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + new_lock_period);

        let caller = self.blockchain().get_caller();
        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);

        let new_locked_tokens = if self.is_new_token(payment.token_nonce) {
            self.extend_new_token_period(payment, new_unlock_epoch, &mut energy, current_epoch)
        } else {
            self.extend_old_token_period(payment, new_unlock_epoch, &mut energy, current_epoch)
        };

        self.set_energy_entry(&caller, energy);

        self.send().direct_esdt(
            &caller,
            &new_locked_tokens.token_identifier,
            new_locked_tokens.token_nonce,
            &new_locked_tokens.amount,
        );

        new_locked_tokens
    }

    fn extend_new_token_period(
        &self,
        payment: EsdtTokenPayment,
        new_unlock_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
        current_epoch: Epoch,
    ) -> EsdtTokenPayment {
        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        require!(
            new_unlock_epoch > attributes.unlock_epoch,
            "New lock period must be longer than the current one."
        );

        energy.update_after_unlock_any(&payment.amount, attributes.unlock_epoch, current_epoch);
        energy.add_after_token_lock(&payment.amount, new_unlock_epoch, current_epoch);

        let unlocked_tokens = self.unlock_tokens_unchecked(payment, &attributes);
        let output_tokens = self.lock_tokens(unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(output_tokens)
    }
}
