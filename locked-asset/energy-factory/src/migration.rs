multiversx_sc::imports!();

use crate::energy::Energy;
use common_structs::{Epoch, UnlockEpochAmountPairs};
use math::safe_sub;
use simple_lock::error_messages::INVALID_PAYMENTS_ERR_MSG;
use unwrappable::Unwrappable;

const TOKEN_MIGRATION_LOCK_EPOCHS_FACTOR: u64 = 4;

#[multiversx_sc::module]
pub trait SimpleLockMigrationModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + multiversx_sc_modules::pause::PauseModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    /// Sets the energy amounts and token amounts for users. Overwrites any existing values.
    /// Expects any number of pairs of (user address, token amount, energy amount).
    #[only_owner]
    #[endpoint(setEnergyForOldTokens)]
    fn set_energy_for_old_tokens(
        &self,
        users_energy: MultiValueEncoded<MultiValue3<ManagedAddress, BigUint, BigInt>>,
    ) {
        self.require_paused();

        let user_updated_energy_mapper = self.user_updated_old_tokens_energy();
        let current_epoch = self.blockchain().get_block_epoch();
        for user_energy in users_energy {
            let (user, total_locked_tokens, energy_amount) = user_energy.into_tuple();
            let new_energy = Energy::new(energy_amount, current_epoch, total_locked_tokens);
            self.set_energy_entry(&user, new_energy);
            user_updated_energy_mapper.add(&user);
        }
    }

    #[endpoint(updateEnergyAfterOldTokenUnlock)]
    fn update_energy_after_old_token_unlock(
        &self,
        original_caller: ManagedAddress,
        initial_epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
        final_epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
    ) {
        if self.blockchain().is_smart_contract(&original_caller) {
            return;
        }

        self.require_not_paused();
        self.require_caller_old_factory();
        self.require_old_tokens_energy_was_updated(&original_caller);

        let current_epoch = self.blockchain().get_block_epoch();

        self.update_energy(&original_caller, |energy: &mut Energy<Self::Api>| {
            for pair in initial_epoch_amount_pairs.pairs {
                energy.update_after_unlock_any(&pair.amount, pair.epoch, current_epoch);
            }
            for pair in final_epoch_amount_pairs.pairs {
                energy.add_after_token_lock(&pair.amount, pair.epoch, current_epoch);
            }
        });
    }

    #[payable("*")]
    #[endpoint(migrateOldTokens)]
    fn migrate_old_tokens(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        self.require_not_paused();

        let caller = self.blockchain().get_caller();
        let is_smart_contract_address = self.blockchain().is_smart_contract(&caller);

        if !is_smart_contract_address {
            self.require_old_tokens_energy_was_updated(&caller);
        }

        let payments = self.get_non_empty_payments();
        let current_epoch = self.blockchain().get_block_epoch();
        let legacy_token_id = self.legacy_locked_token_id().get();

        let mut output_payments = ManagedVec::new();
        self.update_energy(&caller, |energy| {
            for payment in &payments {
                require!(
                    payment.token_identifier == legacy_token_id,
                    INVALID_PAYMENTS_ERR_MSG
                );

                let new_token = self.migrate_single_old_token(
                    payment,
                    is_smart_contract_address,
                    current_epoch,
                    energy,
                );
                output_payments.push(new_token);
            }
        });

        self.send().direct_multi(&caller, &output_payments);

        output_payments.into()
    }

    fn migrate_single_old_token(
        &self,
        payment: EsdtTokenPayment,
        is_smart_contract_address: bool,
        current_epoch: Epoch,
        energy: &mut Energy<Self::Api>,
    ) -> EsdtTokenPayment {
        let attributes = self.decode_legacy_token(&payment.token_identifier, payment.token_nonce);
        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        let unlock_epoch_amount_pairs = attributes.get_unlock_amounts_per_epoch(&payment.amount);
        let new_unlock_epoch = self
            .calculate_new_unlock_epoch_for_old_token(&unlock_epoch_amount_pairs, current_epoch);
        for epoch_amount_pair in unlock_epoch_amount_pairs.pairs {
            if is_smart_contract_address {
                energy.add_after_token_lock(
                    &epoch_amount_pair.amount,
                    new_unlock_epoch,
                    current_epoch,
                );
            } else {
                energy.update_after_unlock_epoch_change(
                    &epoch_amount_pair.amount,
                    epoch_amount_pair.epoch,
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

    fn calculate_new_unlock_epoch_for_old_token(
        &self,
        unlock_epoch_amount_pairs: &UnlockEpochAmountPairs<Self::Api>,
        current_epoch: Epoch,
    ) -> Epoch {
        let mut weighted_epochs_sum = BigUint::zero();
        let mut weight_sum = BigUint::zero();
        for epoch_amount_pair in &unlock_epoch_amount_pairs.pairs {
            let lock_epochs_remaining = safe_sub(epoch_amount_pair.epoch, current_epoch);
            weighted_epochs_sum += &epoch_amount_pair.amount * lock_epochs_remaining;
            weight_sum += &epoch_amount_pair.amount;
        }

        let base_lock_epochs_biguint = weighted_epochs_sum / weight_sum;
        let base_lock_epochs = base_lock_epochs_biguint
            .to_u64()
            .unwrap_or_panic::<Self::Api>();

        let new_tentative_unlock_epoch = base_lock_epochs * TOKEN_MIGRATION_LOCK_EPOCHS_FACTOR;
        let lock_options = self.get_lock_options();
        let max_lock_option = lock_options.last().unwrap_or_panic::<Self::Api>();
        let mut new_unlock_epoch =
            core::cmp::min(new_tentative_unlock_epoch, max_lock_option.lock_epochs);

        let min_lock_period = self.min_migrated_token_locked_period().get();
        new_unlock_epoch = core::cmp::max(new_unlock_epoch, min_lock_period);

        self.unlock_epoch_to_start_of_month_upper_estimate(current_epoch + new_unlock_epoch)
    }

    fn require_caller_old_factory(&self) {
        let caller = self.blockchain().get_caller();
        let old_factory_address = self.old_locked_asset_factory_address().get();
        require!(
            caller == old_factory_address,
            "May only call this through old factory SC"
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

    #[storage_mapper("minMigratedTokenLockedPeriod")]
    fn min_migrated_token_locked_period(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("userUpdatedOldTokensEnergy")]
    fn user_updated_old_tokens_energy(&self) -> WhitelistMapper<ManagedAddress>;
}
