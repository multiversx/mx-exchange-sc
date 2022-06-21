#![no_std]

elrond_wasm::imports!();

pub mod error_messages;
pub mod farm_interactions;
pub mod locked_token;
pub mod lp_interactions;
pub mod proxy_farm;
pub mod proxy_lp;
pub mod token_attributes;

use crate::locked_token::LockedTokenAttributes;
use error_messages::*;

#[elrond_wasm::contract]
pub trait SimpleLock:
    locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + proxy_lp::ProxyLpModule
    + proxy_farm::ProxyFarmModule
    + lp_interactions::LpInteractionsModule
    + farm_interactions::FarmInteractionsModule
    + token_attributes::TokenAttributesModule
{
    #[init]
    fn init(&self) {}

    /// Locks any token (including EGLD) until `unlock_epoch` and receive meta ESDT LOCKED tokens.
    /// on a 1:1 ratio. If unlock epoch has already passed, the original tokens are sent instead.
    ///
    /// Expected payment: Any token
    ///
    /// Arguments:
    /// - unlock epoch - the epoch from which the LOCKED token holder may call the unlock endpoint
    /// - opt_destination - OPTIONAL: destination address for the LOCKED tokens
    ///
    /// Output payments: LOCKED tokens (or original payment if current_epoch >= unlock_epoch)
    #[payable("*")]
    #[endpoint(lockTokens)]
    fn lock_tokens(
        &self,
        unlock_epoch: u64,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().egld_or_single_esdt();
        require!(payment.amount > 0, NO_PAYMENT_ERR_MSG);

        let dest_address = self.dest_from_optional(opt_destination);
        let current_epoch = self.blockchain().get_block_epoch();
        if current_epoch >= unlock_epoch {
            self.send().direct(
                &dest_address,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );

            return payment;
        }

        let attributes = LockedTokenAttributes {
            original_token_id: payment.token_identifier.clone(),
            original_token_nonce: payment.token_nonce,
            unlock_epoch,
        };
        let locked_token_mapper = self.locked_token();
        let sft_nonce = self.get_or_create_nonce_for_attributes(
            &locked_token_mapper,
            &payment.token_identifier.into_name(),
            &attributes,
        );
        self.locked_token()
            .nft_add_quantity_and_send(&dest_address, sft_nonce, payment.amount)
            .into()
    }

    /// Unlock tokens, previously locked with the `lockTokens` endpoint
    ///
    /// Expected payment: LOCKED tokens
    ///
    /// Arguments:
    /// - opt_destination - OPTIONAL: destination address for the unlocked tokens
    ///
    /// Output payments: the originally locked tokens
    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens(
        &self,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        require!(payment.amount > 0, NO_PAYMENT_ERR_MSG);

        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Cannot unlock yet"
        );

        locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let dest_address = self.dest_from_optional(opt_destination);
        self.send().direct(
            &dest_address,
            &attributes.original_token_id,
            attributes.original_token_nonce,
            &payment.amount,
        );

        EgldOrEsdtTokenPayment::new(
            attributes.original_token_id,
            attributes.original_token_nonce,
            payment.amount,
        )
    }

    fn dest_from_optional(&self, opt_destination: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_destination {
            OptionalValue::Some(dest) => dest,
            OptionalValue::None => self.blockchain().get_caller(),
        }
    }
}
