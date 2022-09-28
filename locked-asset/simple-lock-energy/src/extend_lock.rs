elrond_wasm::imports!();

use common_structs::{Epoch, OldLockedTokenAttributes};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::energy::Energy;

const MAX_MILESTONES_IN_OLD_TOKEN_SCHEDULE: usize = 64;
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
    fn lock_and_send_by_token_type(
        &self,
        dest_address: &ManagedAddress,
        payment: EsdtTokenPayment,
        unlock_epoch: Epoch,
        current_epoch: Epoch,
    ) -> EsdtTokenPayment {
        let mut energy = self.get_updated_energy_entry_for_user(dest_address);
        let locked_tokens = if self.is_base_asset_token(&payment.token_identifier) {
            self.lock_base_asset(payment, unlock_epoch, current_epoch, &mut energy)
        } else if self.is_legacy_locked_token(&payment.token_identifier) {
            self.extend_old_token_period(payment, unlock_epoch, current_epoch, &mut energy)
        } else {
            self.locked_token()
                .require_same_token(&payment.token_identifier);

            self.extend_new_token_period(payment, unlock_epoch, current_epoch, &mut energy)
        };

        self.set_energy_entry(&dest_address, energy);

        self.send().direct_esdt(
            &dest_address,
            &locked_tokens.token_identifier,
            locked_tokens.token_nonce,
            &locked_tokens.amount,
        );

        locked_tokens
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
        let locked_token_mapper = self.locked_token();
        let attributes: OldLockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
        let unlock_epoch_amount_pairs = attributes
            .get_unlock_amounts_per_milestone::<MAX_MILESTONES_IN_OLD_TOKEN_SCHEDULE>(
                &payment.amount,
            );

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
}
