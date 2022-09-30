elrond_wasm::imports!();

use common_structs::{Epoch, OldLockedTokenAttributes};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::energy::Energy;

static INVALID_EXTEND_PERIOD_ARG_ERR_MSG: &[u8] =
    b"New lock period must be longer than the current one";

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
    fn lock_by_token_type(
        &self,
        dest_address: &ManagedAddress,
        payment: EsdtTokenPayment,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) -> EsdtTokenPayment {
        let output_payment = self.update_energy(dest_address, |energy: &mut Energy<Self::Api>| {
            let payment_clone = payment.clone();
            if self.is_base_asset_token(&payment.token_identifier) {
                self.lock_base_asset(payment_clone, unlock_epoch, current_epoch, energy)
            } else if self.is_legacy_locked_token(&payment.token_identifier) {
                self.require_address_is_caller(dest_address);

                self.extend_old_token_period(payment_clone, unlock_epoch, current_epoch, energy)
            } else {
                self.require_address_is_caller(dest_address);
                self.locked_token()
                    .require_same_token(&payment.token_identifier);

                self.extend_new_token_period(payment_clone, unlock_epoch, current_epoch, energy)
            }
        });

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        output_payment
    }

    fn lock_base_asset(
        &self,
        payment: EsdtTokenPayment,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let output_tokens = self.lock_tokens(payment.into(), unlock_epoch);
        energy.add_after_token_lock(&output_tokens.amount, unlock_epoch, current_epoch);

        self.to_esdt_payment(output_tokens)
    }

    fn extend_old_token_period(
        &self,
        payment: EsdtTokenPayment,
        new_unlock_epoch: Epoch,
        current_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let own_sc_address = self.blockchain().get_sc_address();
        let old_token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &payment.token_identifier,
            payment.token_nonce,
        );
        let attributes: OldLockedTokenAttributes<Self::Api> = old_token_data.decode_attributes();
        let unlock_epoch_amount_pairs =
            attributes.get_unlock_amounts_per_epoch(&payment.amount);

        for epoch_amount_pair in unlock_epoch_amount_pairs.pairs {
            require!(
                epoch_amount_pair.epoch < new_unlock_epoch,
                INVALID_EXTEND_PERIOD_ARG_ERR_MSG
            );

            energy.update_after_extend(
                &epoch_amount_pair.amount,
                epoch_amount_pair.epoch,
                new_unlock_epoch,
                current_epoch,
            );
        }

        let base_asset = EgldOrEsdtTokenIdentifier::esdt(self.base_asset_token_id().get());
        let original_unlocked_tokens = EgldOrEsdtTokenPayment {
            token_identifier: base_asset,
            token_nonce: 0,
            amount: payment.amount,
        };
        let new_locked_tokens = self.lock_tokens(original_unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(new_locked_tokens)
    }

    fn extend_new_token_period(
        &self,
        payment: EsdtTokenPayment,
        new_unlock_epoch: Epoch,
        current_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        require!(
            new_unlock_epoch > attributes.unlock_epoch,
            INVALID_EXTEND_PERIOD_ARG_ERR_MSG
        );

        energy.update_after_extend(
            &payment.amount,
            attributes.unlock_epoch,
            new_unlock_epoch,
            current_epoch,
        );

        let unlocked_tokens = self.unlock_tokens_unchecked(payment, &attributes);
        let output_tokens = self.lock_tokens(unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(output_tokens)
    }

    fn require_address_is_caller(&self, address: &ManagedAddress) {
        let caller = self.blockchain().get_caller();
        require!(
            address == &caller,
            "May not use the optional destination argument here"
        );
    }
}
