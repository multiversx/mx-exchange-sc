elrond_wasm::imports!();

use crate::{error_messages::NO_PAYMENT_ERR_MSG, locked_token::LockedTokenAttributes};

#[elrond_wasm::module]
pub trait BasicLockUnlock:
    crate::locked_token::LockedTokenModule
    + crate::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn lock_tokens(
        &self,
        payment: EgldOrEsdtTokenPayment<Self::Api>,
        unlock_epoch: u64,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        require!(payment.amount > 0, NO_PAYMENT_ERR_MSG);

        let current_epoch = self.blockchain().get_block_epoch();
        if current_epoch >= unlock_epoch {
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
            .nft_add_quantity(sft_nonce, payment.amount)
            .into()
    }

    fn lock_and_send(
        &self,
        to: &ManagedAddress,
        payment: EgldOrEsdtTokenPayment<Self::Api>,
        unlock_epoch: u64,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let out_payment = self.lock_tokens(payment, unlock_epoch);
        self.send().direct(
            to,
            &out_payment.token_identifier,
            out_payment.token_nonce,
            &out_payment.amount,
        );

        out_payment
    }

    fn unlock_tokens(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
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

        EgldOrEsdtTokenPayment::new(
            attributes.original_token_id,
            attributes.original_token_nonce,
            payment.amount,
        )
    }

    fn unlock_and_send(
        &self,
        to: &ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let out_payment = self.unlock_tokens(payment);
        self.send().direct(
            to,
            &out_payment.token_identifier,
            out_payment.token_nonce,
            &out_payment.amount,
        );

        out_payment
    }
}
