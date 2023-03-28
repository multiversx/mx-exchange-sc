multiversx_sc::imports!();

use crate::energy::Energy;
use common_structs::Epoch;

#[multiversx_sc::module]
pub trait VirtualLockModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::events::EventsModule
    + crate::migration::SimpleLockMigrationModule
    + multiversx_sc_modules::pause::PauseModule
    + utils::UtilsModule
    + crate::extend_lock::ExtendLockModule
    + sc_whitelist_module::SCWhitelistModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    #[endpoint(lockVirtual)]
    fn lock_virtual(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        lock_epochs: Epoch,
        dest_address: ManagedAddress,
        energy_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        require!(
            self.is_base_asset_token(&token_id),
            "May only lock the base asset token"
        );
        require!(amount > 0, "Amount cannot be 0");
        self.require_is_listed_lock_option(lock_epochs);

        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_epoch = self.unlock_epoch_to_start_of_month(current_epoch + lock_epochs);

        require!(
            unlock_epoch > current_epoch,
            "Unlock epoch must be greater than the current epoch"
        );

        let locked_tokens =
            self.update_energy(&energy_address, |energy: &mut Energy<Self::Api>| {
                self.lock_base_asset(
                    EsdtTokenPayment::new(token_id, 0, amount),
                    unlock_epoch,
                    current_epoch,
                    energy,
                )
            });
        self.send().direct_esdt(
            &dest_address,
            &locked_tokens.token_identifier,
            locked_tokens.token_nonce,
            &locked_tokens.amount,
        );

        locked_tokens
    }
}
