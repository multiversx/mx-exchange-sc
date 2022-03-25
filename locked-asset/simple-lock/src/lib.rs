#![no_std]

use crate::locked_token::LockedTokenAttributes;

elrond_wasm::imports!();

pub mod locked_token;

#[elrond_wasm::contract]
pub trait SimpleLock:
    locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        require!(payment_amount > 0, "No payment");

        let dest_address = self.dest_from_optional(opt_destination);
        let attributes = LockedTokenAttributes {
            original_token_id: payment_token,
            original_token_nonce: payment_nonce,
            unlock_epoch,
        };

        self.locked_token()
            .nft_create_and_send(&dest_address, payment_amount, &attributes)
    }

    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens(
        &self,
        #[var_args] opt_destination: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let (payment_token, payment_nonce, payment_amount) = self.call_value().payment_as_tuple();
        require!(payment_amount > 0, "No payment");

        let locked_token_id = self.locked_token().get_token_id();
        require!(payment_token == locked_token_id, "Invalid token");

        let attributes: LockedTokenAttributes<Self::Api> =
            self.locked_token().get_token_attributes(payment_nonce);
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Cannot unlock yet"
        );

        self.locked_token().nft_burn(payment_nonce, &payment_amount);

        let dest_address = self.dest_from_optional(opt_destination);
        self.send().direct(
            &dest_address,
            &attributes.original_token_id,
            attributes.original_token_nonce,
            &payment_amount,
            &[],
        );

        EsdtTokenPayment::new(
            attributes.original_token_id,
            attributes.original_token_nonce,
            payment_amount,
        )
    }

    fn dest_from_optional(&self, opt_destination: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_destination {
            OptionalValue::Some(dest) => dest,
            OptionalValue::None => self.blockchain().get_caller(),
        }
    }
}
