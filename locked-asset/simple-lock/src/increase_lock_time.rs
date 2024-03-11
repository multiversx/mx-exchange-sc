use common_structs::Epoch;

use crate::locked_token::LockedTokenAttributes;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait IncreaseLockTimeModule:
    crate::basic_lock_unlock::BasicLockUnlock
    + crate::locked_token::LockedTokenModule
    + crate::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(increaseLockTime)]
    fn increase_lock_time(&self, new_unlock_epoch: Epoch) -> EsdtTokenPayment {
        let payment = self.call_value().single_esdt();
        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
        require!(
            new_unlock_epoch > attributes.unlock_epoch,
            "New unlock epoch must be higher"
        );

        let current_epoch = self.blockchain().get_block_epoch();
        require!(new_unlock_epoch > current_epoch, "Invalid new unlock epoch");

        locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let new_attributes = LockedTokenAttributes {
            original_token_id: attributes.original_token_id,
            original_token_nonce: attributes.original_token_nonce,
            unlock_epoch: new_unlock_epoch,
        };
        let caller = self.blockchain().get_caller();

        locked_token_mapper.nft_create_and_send(&caller, payment.amount, &new_attributes)
    }
}
