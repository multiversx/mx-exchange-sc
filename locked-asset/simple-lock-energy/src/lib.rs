#![no_std]

elrond_wasm::imports!();

pub mod token_whitelist;

#[elrond_wasm::contract]
pub trait SimpleLockEnergy:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + token_whitelist::TokenWhitelistModule
{
    /// Args:
    /// - base_asset_token_id: The only token that is accepted for the lockTokens endpoint.
    #[init]
    fn init(&self, base_asset_token_id: TokenIdentifier) {
        self.require_valid_token_id(&base_asset_token_id);
        self.base_asset_token_id().set(&base_asset_token_id);
    }

    /// Locks a whitelisted token until `unlock_epoch` and receive meta ESDT LOCKED tokens.
    /// on a 1:1 ratio. If unlock epoch has already passed, the original tokens are sent instead.
    ///
    /// Expected payment: A whitelisted token
    ///
    /// Arguments:
    /// - unlock epoch - the epoch from which the LOCKED token holder may call the unlock endpoint
    /// - opt_destination - OPTIONAL: destination address for the LOCKED tokens. Default is caller.
    ///
    /// Output payments: LOCKED tokens (or original payment if current_epoch >= unlock_epoch)
    #[payable("*")]
    #[endpoint(lockTokens)]
    fn lock_tokens_endpoint(
        &self,
        unlock_epoch: u64,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().single_esdt();
        self.require_is_base_asset_token(&payment.token_identifier);

        let dest_address = self.dest_from_optional(opt_destination);
        self.lock_and_send(&dest_address, payment.into(), unlock_epoch)
    }

    /// Unlock tokens, previously locked with the `lockTokens` endpoint
    ///
    /// Expected payment: LOCKED tokens
    ///
    /// Arguments:
    /// - opt_destination - OPTIONAL: destination address for the unlocked tokens. Default is caller.
    ///
    /// Output payments: the originally locked tokens
    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens_endpoint(
        &self,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().single_esdt();
        let dest_address = self.dest_from_optional(opt_destination);
        self.unlock_and_send(&dest_address, payment)
    }

    fn dest_from_optional(&self, opt_destination: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_destination {
            OptionalValue::Some(dest) => dest,
            OptionalValue::None => self.blockchain().get_caller(),
        }
    }
}
