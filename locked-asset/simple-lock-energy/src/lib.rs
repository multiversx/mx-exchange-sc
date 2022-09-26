#![no_std]

elrond_wasm::imports!();

pub mod energy;
pub mod events;
pub mod extend_lock;
pub mod lock_options;
pub mod migration;
pub mod old_token_actions;
pub mod old_token_nonces;
pub mod token_whitelist;
pub mod unlock_with_penalty;
pub mod util;

use common_structs::Epoch;
use energy::Energy;
use simple_lock::{error_messages::NO_PAYMENT_ERR_MSG, locked_token::LockedTokenAttributes};

#[elrond_wasm::contract]
pub trait SimpleLockEnergy:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + token_whitelist::TokenWhitelistModule
    + energy::EnergyModule
    + lock_options::LockOptionsModule
    + unlock_with_penalty::UnlockWithPenaltyModule
    + extend_lock::ExtendLockModule
    + util::UtilModule
    + migration::SimpleLockMigrationModule
    + events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
    + old_token_nonces::OldTokenNonces
    + old_token_actions::OldTokenActions
{
    /// Args:
    /// - base_asset_token_id: The only token that is accepted for the lockTokens endpoint.
    ///     NOTE: The SC also needs the ESDTLocalBurn role for this token.
    /// - min_penalty_percentage / max_penalty_percentage: The penalty for early unlock
    ///     of a token. A token locked for the max period, will have max_penalty_percentage penalty,
    ///     whereas one with 1 epoch left, will have min_penalty_percentage.
    ///     Penalty decreases linearly from max to min, based on the remaining locking period.
    ///     
    ///     Both are values between 0 and 10_000, where 10_000 is 100%.
    /// - fees_burn_percentage: The percentage of fees that are burned.
    ///     The rest are sent to the fees collector
    /// - fees_collector_address
    /// - lock_options: List of epochs. Users may only choose from this list when calling lockTokens
    #[init]
    fn init(
        &self,
        base_asset_token_id: TokenIdentifier,
        min_penalty_percentage: u16,
        max_penalty_percentage: u16,
        fees_burn_percentage: u16,
        fees_collector_address: ManagedAddress,
        lock_options: MultiValueEncoded<Epoch>,
    ) {
        self.require_valid_token_id(&base_asset_token_id);

        self.base_asset_token_id().set(&base_asset_token_id);
        self.set_penalty_percentage(min_penalty_percentage, max_penalty_percentage);
        self.set_fees_burn_percentage(fees_burn_percentage);
        self.set_fees_collector_address(fees_collector_address);
        self.add_lock_options(lock_options);
        self.set_paused(true);
    }

    /// Locks a whitelisted token until `unlock_epoch` and receive meta ESDT LOCKED tokens
    /// on a 1:1 ratio.
    ///
    /// Expected payment: A whitelisted token
    ///
    /// Arguments:
    /// - lock_epochs - Number of epochs for which the tokens are locked for.
    ///     Caller may only choose from the available options,
    ///     which can be seen by querying getLockOptions
    /// - opt_destination - OPTIONAL: destination address for the LOCKED tokens. Default is caller.
    ///
    /// Output payment: LOCKED tokens
    #[payable("*")]
    #[endpoint(lockTokens)]
    fn lock_tokens_endpoint(
        &self,
        lock_epochs: u64,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_is_listed_lock_option(lock_epochs);

        let payment = self.call_value().single_esdt();
        self.require_is_base_asset_token(&payment.token_identifier);

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + lock_epochs);

        let dest_address = self.dest_from_optional(opt_destination);
        let output_tokens = self.lock_and_send(&dest_address, payment.into(), unlock_epoch);

        let mut energy = self.get_updated_energy_entry_for_user(&dest_address, current_epoch);
        energy.add_after_token_lock(&output_tokens.amount, unlock_epoch, current_epoch);
        self.set_energy_entry(&dest_address, energy);

        self.to_esdt_payment(output_tokens)
    }

    /// Unlock tokens, previously locked with the `lockTokens` endpoint
    ///
    /// Expected payments: LOCKED tokens
    ///
    /// Output payments: the originally locked tokens and new LOCKED tokens if necessary.
    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens_endpoint(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        self.require_not_paused();

        let payments = self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), NO_PAYMENT_ERR_MSG);

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();

        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);
        let mut output_payments = ManagedVec::new();
        for payment in &payments {
            if self.is_new_token(payment.token_nonce) {
                let unlocked_tokens = self.unlock_new_token(payment, &mut energy, current_epoch);
                output_payments.push(unlocked_tokens);
            } else {
                let pair = self.unlock_old_token(payment, &mut energy, current_epoch);
                output_payments.push(pair.unlocked);

                if let Some(locked_tokens) = pair.opt_locked {
                    output_payments.push(locked_tokens);
                }
            }
        }

        self.set_energy_entry(&caller, energy);
        self.send().direct_multi(&caller, &output_payments);

        output_payments.into()
    }

    fn unlock_new_token(
        &self,
        payment: EsdtTokenPayment,
        energy: &mut Energy<Self::Api>,
        current_epoch: Epoch,
    ) -> EsdtTokenPayment {
        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        let output_tokens = self.unlock_tokens(payment);
        energy.refund_after_token_unlock(
            &output_tokens.amount,
            attributes.unlock_epoch,
            current_epoch,
        );

        self.to_esdt_payment(output_tokens)
    }
}
