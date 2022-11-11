#![no_std]

elrond_wasm::imports!();

pub mod energy;
pub mod events;
pub mod extend_lock;
pub mod fees;
pub mod local_roles;
pub mod lock_options;
pub mod migration;
pub mod penalty;
pub mod token_merging;
pub mod token_whitelist;
pub mod unlock_with_penalty;
pub mod virtual_lock;

use common_structs::{Epoch, Percent};
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;
use token_unstake::DEFAULT_UNBOND_EPOCHS;

use crate::energy::Energy;

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
    + migration::SimpleLockMigrationModule
    + events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
    + local_roles::LocalRolesModule
    + token_merging::TokenMergingModule
    + token_send::TokenSendModule
    + token_unstake::TokenUnstakeModule
    + penalty::LocalPenaltyModule
    + fees::FeesModule
    + utils::UtilsModule
    + virtual_lock::VirtualLockModule
    + sc_whitelist_module::SCWhitelistModule
{
    /// Args:
    /// - base_asset_token_id: The only token that is accepted for the lockTokens endpoint.
    ///     NOTE: The SC also needs the ESDTLocalMint and ESDTLocalBurn roles for this token.
    /// - legacy_token_id: The token ID of the old locked asset.
    ///     NOTE: The SC also needs the NFTBurn role for this token.
    /// - min_penalty_percentage / max_penalty_percentage: The penalty for early unlock
    ///     of a token. A token locked for the max period, will have max_penalty_percentage penalty,
    ///     whereas one with 1 epoch left, will have min_penalty_percentage.
    ///     Penalty decreases linearly from max to min, based on the remaining locking period.
    ///     
    ///     Both are values between 0 and 10_000, where 10_000 is 100%.
    /// - fees_burn_percentage: The percentage of fees that are burned.
    ///     The rest are sent to the fees collector
    /// - fees_collector_address
    /// - lock_options: See `addLockOptions` endpoint doc for details.
    #[init]
    fn init(
        &self,
        base_asset_token_id: TokenIdentifier,
        legacy_token_id: TokenIdentifier,
        fees_burn_percentage: u16,
        fees_collector_address: ManagedAddress,
        old_locked_asset_factory_address: ManagedAddress,
        lock_options: MultiValueEncoded<MultiValue2<Epoch, Percent>>,
    ) {
        self.require_valid_token_id(&base_asset_token_id);
        self.require_valid_token_id(&legacy_token_id);
        self.require_sc_address(&old_locked_asset_factory_address);

        self.base_asset_token_id().set(&base_asset_token_id);
        self.legacy_locked_token_id().set(&legacy_token_id);
        self.set_fees_burn_percentage(fees_burn_percentage);
        self.set_fees_collector_address(fees_collector_address);
        self.old_locked_asset_factory_address()
            .set(&old_locked_asset_factory_address);
        self.add_lock_options(lock_options);
        self.unbond_epochs().set_if_empty(DEFAULT_UNBOND_EPOCHS);
        self.set_paused(true);
    }

    /// Locks a whitelisted token until `unlock_epoch` and receive meta ESDT LOCKED tokens
    /// on a 1:1 ratio. Accepted input tokens:
    /// - base asset token
    /// - old factory token -> extends all periods to the provided option
    /// - previously locked token -> extends period to the provided option
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
        lock_epochs: Epoch,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_is_listed_lock_option(lock_epochs);

        let payment = self.call_value().single_esdt();
        let dest_address = self.dest_from_optional(opt_destination);
        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + lock_epochs);

        let output_tokens =
            self.lock_by_token_type(&dest_address, payment, unlock_epoch, current_epoch);

        self.send().direct_esdt(
            &dest_address,
            &output_tokens.token_identifier,
            output_tokens.token_nonce,
            &output_tokens.amount,
        );

        output_tokens
    }

    /// Unlock tokens, previously locked with the `lockTokens` endpoint
    ///
    /// Expected payments: LOCKED tokens
    ///
    /// Output payments: the originally locked tokens
    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens_endpoint(&self) -> EsdtTokenPayment {
        self.require_not_paused();

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        let locked_token_mapper = self.locked_token();

        let base_asset = self.base_asset_token_id().get();
        let mut output_payment = EsdtTokenPayment::new(base_asset, 0, BigUint::zero());

        self.update_energy(&caller, |energy: &mut Energy<Self::Api>| {
            let payments = self.get_non_empty_payments();
            locked_token_mapper.require_all_same_token(&payments);

            for payment in &payments {
                let attributes: LockedTokenAttributes<Self::Api> =
                    locked_token_mapper.get_token_attributes(payment.token_nonce);

                let unlocked_tokens = self.unlock_tokens(payment);
                energy.refund_after_token_unlock(
                    &unlocked_tokens.amount,
                    attributes.unlock_epoch,
                    current_epoch,
                );

                output_payment.merge_with(self.to_esdt_payment(unlocked_tokens));
            }
        });

        self.send()
            .esdt_local_mint(&output_payment.token_identifier, 0, &output_payment.amount);
        self.send().direct_esdt(
            &caller,
            &output_payment.token_identifier,
            0,
            &output_payment.amount,
        );

        output_payment
    }
}
