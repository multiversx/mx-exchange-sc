elrond_wasm::imports!();

use crate::{energy::Energy, extend_lock::INVALID_EXTEND_PERIOD_ARG_ERR_MSG};
use common_structs::{Epoch, OldLockedTokenAttributes, PaymentsVec, UnlockEpochAmountPairs};
use unwrappable::Unwrappable;

#[elrond_wasm::module]
pub trait SimpleLockMigrationModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + elrond_wasm_modules::pause::PauseModule
    + utils::UtilsModule
{
    #[only_owner]
    #[endpoint(updateEnergyForOldTokens)]
    fn update_energy_for_old_tokens(
        &self,
        user: ManagedAddress,
        total_locked_tokens: BigUint,
        energy_amount: BigUint,
    ) {
        self.require_old_tokens_energy_not_updated(&user);

        self.update_energy(&user, |energy: &mut Energy<Self::Api>| {
            energy.add_energy_raw(total_locked_tokens, energy_amount);
        });

        self.user_updated_old_tokens_energy().add(&user);
    }

    #[endpoint(updateEnergyAfterOldTokenUnlock)]
    fn update_energy_after_old_token_unlock(
        &self,
        original_caller: ManagedAddress,
        epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
    ) {
        self.require_caller_old_factory();

        let old_token_energy_updated = self
            .user_updated_old_tokens_energy()
            .contains(&original_caller);
        if !old_token_energy_updated {
            return;
        }

        self.update_energy(&original_caller, |energy: &mut Energy<Self::Api>| {
            let current_epoch = self.blockchain().get_block_epoch();
            for pair in epoch_amount_pairs.pairs {
                energy.refund_after_token_unlock(&pair.amount, pair.epoch, current_epoch);
            }
        });
    }

    #[payable("*")]
    #[endpoint(migrateOldTokens)]
    fn migrate_old_token(&self) -> PaymentsVec<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let current_epoch = self.blockchain().get_block_epoch();
        let own_sc_address = self.blockchain().get_sc_address();

        let mut output_payments = PaymentsVec::new();
        self.update_energy(&caller, |energy| {
            for payment in &payments {
                require!(
                    self.is_legacy_locked_token(&payment.token_identifier),
                    "Invalid token"
                );

                let old_token_data = self.blockchain().get_esdt_token_data(
                    &own_sc_address,
                    &payment.token_identifier,
                    payment.token_nonce,
                );
                let attributes: OldLockedTokenAttributes<Self::Api> =
                    old_token_data.decode_attributes();
                let lock_epochs = self.calculate_optimal_lock_option_for_old_token(&attributes);
                let new_unlock_epoch = current_epoch + lock_epochs;
                let locked_token = self.extend_old_token_period(
                    &caller,
                    payment.clone(),
                    new_unlock_epoch,
                    current_epoch,
                    energy,
                );
                output_payments.push(locked_token);

                self.send().esdt_local_burn(
                    &payment.token_identifier,
                    payment.token_nonce,
                    &payment.amount,
                );
            }
        });

        self.send().direct_multi(&caller, &output_payments);

        output_payments
    }

    fn calculate_optimal_lock_option_for_old_token(
        &self,
        attributes: &OldLockedTokenAttributes<Self::Api>,
    ) -> Epoch {
        let lock_options = self.get_lock_options();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut max_old_lock_epochs = 0;
        for milestone in &attributes.unlock_schedule.unlock_milestones {
            let lock_epochs = if milestone.unlock_epoch > current_epoch {
                milestone.unlock_epoch - current_epoch
            } else {
                0
            };
            if lock_epochs > max_old_lock_epochs {
                max_old_lock_epochs = lock_epochs;
            }
        }

        for lock_option in &lock_options {
            if lock_option.lock_epochs >= max_old_lock_epochs {
                return lock_option.lock_epochs;
            }
        }

        lock_options
            .last()
            .unwrap_or_panic::<Self::Api>()
            .lock_epochs
    }

    fn extend_old_token_period(
        &self,
        caller: &ManagedAddress,
        payment: EsdtTokenPayment,
        new_unlock_epoch: Epoch,
        current_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let old_token_energy_updated = self.user_updated_old_tokens_energy().contains(caller);

        let own_sc_address = self.blockchain().get_sc_address();
        let old_token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &payment.token_identifier,
            payment.token_nonce,
        );
        let attributes: OldLockedTokenAttributes<Self::Api> = old_token_data.decode_attributes();
        let unlock_epoch_amount_pairs = attributes.get_unlock_amounts_per_epoch(&payment.amount);
        for epoch_amount_pair in unlock_epoch_amount_pairs.pairs {
            require!(
                epoch_amount_pair.epoch < new_unlock_epoch,
                INVALID_EXTEND_PERIOD_ARG_ERR_MSG
            );

            if old_token_energy_updated {
                energy.update_after_extend(
                    &epoch_amount_pair.amount,
                    epoch_amount_pair.epoch,
                    new_unlock_epoch,
                    current_epoch,
                );
            } else {
                energy.add_after_token_lock(
                    &epoch_amount_pair.amount,
                    new_unlock_epoch,
                    current_epoch,
                );
            }
        }

        let base_asset = EgldOrEsdtTokenIdentifier::esdt(self.base_asset_token_id().get());
        let original_unlocked_tokens = EgldOrEsdtTokenPayment::new(base_asset, 0, payment.amount);
        let new_locked_tokens = self.lock_tokens(original_unlocked_tokens, new_unlock_epoch);

        self.to_esdt_payment(new_locked_tokens)
    }

    fn require_caller_old_factory(&self) {
        let caller = self.blockchain().get_caller();
        let old_factory_address = self.old_locked_asset_factory_address().get();
        require!(
            caller == old_factory_address,
            "May only call this through old factory SC"
        );
    }

    fn require_old_tokens_energy_not_updated(&self, address: &ManagedAddress) {
        require!(
            !self.user_updated_old_tokens_energy().contains(address),
            "Energy for old tokens already updated"
        );
    }

    fn require_old_tokens_energy_was_updated(&self, address: &ManagedAddress) {
        require!(
            self.user_updated_old_tokens_energy().contains(address),
            "Must have energy updated for old tokens first"
        );
    }

    #[storage_mapper("oldLockedAssetFactoryAddress")]
    fn old_locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("userUpdatedOldTokensEnergy")]
    fn user_updated_old_tokens_energy(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
