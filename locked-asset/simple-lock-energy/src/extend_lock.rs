elrond_wasm::imports!();

use common_structs::Epoch;
use simple_lock::locked_token::LockedTokenAttributes;

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
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
{
    /// Extend locking period of a previously locked token.
    /// new_lock_period must still be one of the available options.
    /// No penalty is received for performing this action.
    #[payable("*")]
    #[endpoint(extendLockingPeriod)]
    fn extend_locking_period(&self, new_lock_period: Epoch) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_is_listed_lock_option(new_lock_period);

        let payment = self.call_value().single_esdt();
        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        let new_unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + new_lock_period);
        require!(
            new_unlock_epoch > attributes.unlock_epoch,
            "New lock period must be longer than the current one."
        );

        let caller = self.blockchain().get_caller();

        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);
        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);
        energy.add_after_token_lock(&payment.amount, new_unlock_epoch, current_epoch);
        self.set_energy_entry(&caller, energy);

        let unlocked_tokens = self.unlock_tokens_unchecked(payment, &attributes);
        let output_payment = self.lock_and_send(&caller, unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(output_payment)
    }
}
