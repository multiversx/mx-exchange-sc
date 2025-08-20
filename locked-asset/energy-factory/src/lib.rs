#![no_std]

multiversx_sc::imports!();

pub mod energy;
pub mod events;
pub mod extend_lock;
pub mod local_roles;
pub mod lock_options;
pub mod lock_options_endpoints;
pub mod locked_token_transfer;
pub mod migration;
pub mod penalty;
pub mod token_merging;
pub mod token_whitelist;
pub mod unlock_with_penalty;
pub mod unlocked_token_transfer;
pub mod unstake;
pub mod virtual_lock;

use common_structs::{Epoch, Percent};
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;
use unwrappable::Unwrappable;

use crate::energy::Energy;

#[multiversx_sc::contract]
pub trait SimpleLockEnergy:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + token_whitelist::TokenWhitelistModule
    + energy::EnergyModule
    + lock_options::LockOptionsModule
    + lock_options_endpoints::LockOptionsEndpointsModule
    + unlock_with_penalty::UnlockWithPenaltyModule
    + unstake::UnstakeModule
    + extend_lock::ExtendLockModule
    + migration::SimpleLockMigrationModule
    + events::EventsModule
    + multiversx_sc_modules::pause::PauseModule
    + local_roles::LocalRolesModule
    + token_merging::TokenMergingModule
    + penalty::LocalPenaltyModule
    + utils::UtilsModule
    + virtual_lock::VirtualLockModule
    + sc_whitelist_module::SCWhitelistModule
    + locked_token_transfer::LockedTokenTransferModule
    + unlocked_token_transfer::UnlockedTokenTransferModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    /// Args:
    /// - base_asset_token_id: The only token that is accepted for the lockTokens endpoint.
    ///     NOTE: The SC also needs the ESDTLocalMint and ESDTLocalBurn roles for this token.
    /// - legacy_token_id: The token ID of the old locked asset.
    ///     NOTE: The SC also needs the NFTBurn role for this token.
    /// - old_locked_asset_factory_address
    /// - min_migrated_token_locked_period - The minimum number of epochs that
    ///     a migrated old LKMEX token will be locked for after the average is calculated
    /// - lock_options: See `addLockOptions` endpoint doc for details.
    #[init]
    fn init(
        &self,
        base_asset_token_id: TokenIdentifier,
        legacy_token_id: TokenIdentifier,
        old_locked_asset_factory_address: ManagedAddress,
        min_migrated_token_locked_period: Epoch,
        lock_options: MultiValueEncoded<MultiValue2<Epoch, Percent>>,
    ) {
        self.require_valid_token_id(&base_asset_token_id);
        self.require_valid_token_id(&legacy_token_id);
        self.require_sc_address(&old_locked_asset_factory_address);

        self.add_lock_options(lock_options);

        let all_lock_options = self.get_lock_options();
        let max_lock_option = all_lock_options.last().unwrap_or_panic::<Self::Api>();
        require!(
            min_migrated_token_locked_period <= max_lock_option.lock_epochs,
            "Invalid min epoch for migrated token"
        );
        self.min_migrated_token_locked_period()
            .set(min_migrated_token_locked_period);

        self.base_asset_token_id()
            .set_if_empty(&base_asset_token_id);
        self.legacy_locked_token_id().set_if_empty(&legacy_token_id);
        self.old_locked_asset_factory_address()
            .set_if_empty(&old_locked_asset_factory_address);

        self.set_paused(true);
    }

    #[upgrade]
    fn upgrade(&self) {}

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

        let payment = self.call_value().single_esdt().clone();
        let dest_address = self.dest_from_optional(opt_destination);
        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + lock_epochs);
        require!(
            unlock_epoch > current_epoch,
            "Unlock epoch must be greater than the current epoch"
        );

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

                let unlocked_tokens = self.unlock_tokens(payment.clone().into_tuple().into());
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

    /// Used internally by proxy-dex
    #[payable("*")]
    #[endpoint(extendLockPeriod)]
    fn extend_lock_period(&self, lock_epochs: Epoch, user: ManagedAddress) -> EsdtTokenPayment {
        self.require_not_paused();
        self.require_is_listed_lock_option(lock_epochs);

        let caller = self.blockchain().get_caller();
        require!(
            self.token_transfer_whitelist().contains(&caller),
            "May not call this endpoint. Use lockTokens instead"
        );

        let payment = self.call_value().single_esdt();
        self.locked_token()
            .require_same_token(&payment.token_identifier);

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + lock_epochs);
        require!(
            unlock_epoch > current_epoch,
            "Unlock epoch must be greater than the current epoch"
        );

        let output_tokens = self.update_energy(&user, |energy: &mut Energy<Self::Api>| {
            self.extend_new_token_period(payment.clone(), unlock_epoch, current_epoch, energy)
        });

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        self.send().direct_esdt(
            &caller,
            &output_tokens.token_identifier,
            output_tokens.token_nonce,
            &output_tokens.amount,
        );

        output_tokens
    }

    #[only_owner]
    #[endpoint(adjustUserEnergy)]
    fn adjust_user_energy(
        &self,
        args: MultiValueEncoded<MultiValue3<ManagedAddress, BigInt, BigInt>>,
    ) {
        for arg in args {
            let (user, energy_amount, token_amount) = arg.into_tuple();
            require!(!self.user_energy(&user).is_empty(), "User energy not found");
            let old_energy = self.get_updated_energy_entry_for_user(&user);
            let new_energy_amount = old_energy.get_energy_amount_raw() + &energy_amount;
            let new_total_locked_tokens = if token_amount >= 0 {
                old_energy.get_total_locked_tokens() + &token_amount.magnitude()
            } else {
                let token_amount_magnitude = token_amount.magnitude();
                require!(
                    old_energy.get_total_locked_tokens() >= &token_amount_magnitude,
                    "Insufficient locked tokens"
                );
                old_energy.get_total_locked_tokens() - &token_amount_magnitude
            };

            let current_epoch = self.blockchain().get_block_epoch();
            let new_energy = Energy::new(new_energy_amount, current_epoch, new_total_locked_tokens);

            self.set_energy_entry(&user, new_energy);
        }
    }
}
