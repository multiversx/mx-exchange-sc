#![no_std]

elrond_wasm::imports!();

pub mod error_messages;
pub mod locked_token;
pub mod lp_interactions;
pub mod proxy_lp;

use error_messages::*;
use crate::locked_token::LockedTokenAttributes;

const SFT_EXTRA_AMOUNT_TO_SAVE_ATTRIBUTES: u32 = 1;

#[elrond_wasm::contract]
pub trait SimpleLock:
    locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + proxy_lp::ProxyLpModule
    + lp_interactions::LpInteractionsModule
{
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(lockTokens)]
    fn lock_tokens(
        &self,
        unlock_epoch: u64,
        #[var_args] opt_destination: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let (payment_token, payment_nonce, payment_amount) = self.call_value().payment_as_tuple();
        require!(payment_amount > 0, NO_PAYMENT_ERR_MSG);

        let dest_address = self.dest_from_optional(opt_destination);
        let attributes = LockedTokenAttributes {
            original_token_id: payment_token,
            original_token_nonce: payment_nonce,
            unlock_epoch,
        };

        let mut locked_tokens_payment = self.locked_token().nft_create(
            payment_amount + SFT_EXTRA_AMOUNT_TO_SAVE_ATTRIBUTES,
            &attributes,
        );
        locked_tokens_payment.amount -= SFT_EXTRA_AMOUNT_TO_SAVE_ATTRIBUTES;

        self.send().direct(
            &dest_address,
            &locked_tokens_payment.token_identifier,
            locked_tokens_payment.token_nonce,
            &locked_tokens_payment.amount,
            &[],
        );

        locked_tokens_payment
    }

    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens(
        &self,
        #[var_args] opt_destination: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().payment();
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
            &[],
        );

        EsdtTokenPayment::new(
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
