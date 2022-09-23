elrond_wasm::imports!();

use common_structs::{LockedAssetTokenAttributesEx, Nonce};

#[elrond_wasm::module]
pub trait OldTokenActions:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::lock_options::LockOptionsModule
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
{
    fn unlock_old_token(&self, payment: EsdtTokenPayment) -> EsdtTokenPayment {
        let locked_token_mapper = self.locked_token();
        let attributes: LockedAssetTokenAttributesEx<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
    }

    fn require_new_token(&self, token_nonce: Nonce) {
        require!(
            !self.old_token_nonces().contains(&token_nonce),
            "Only new tokens accepted"
        );
    }

    #[view(getOldTokenNonces)]
    #[storage_mapper("oldTokenNonces")]
    fn old_token_nonces(&self) -> UnorderedSetMapper<Nonce>;
}
