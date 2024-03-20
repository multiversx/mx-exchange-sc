use mergeable::Mergeable;

use crate::locked_token::LockedTokenAttributes;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait MergeTokensModule:
    crate::locked_token::LockedTokenModule
    + crate::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(mergeLockedTokens)]
    fn merge_locked_tokens(&self) -> EsdtTokenPayment {
        let mut payments = self.call_value().all_esdt_transfers().clone_value();
        require!(payments.len() > 1, "Not enough payments");

        let locked_token_mapper = self.locked_token();
        let first_payment = self.pop_first_payment(&mut payments);
        let first_token_attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(first_payment.token_nonce);

        let mut total_locked_tokens = first_payment.amount.clone();
        for payment in &payments {
            let other_token_attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(payment.token_nonce);
            first_token_attributes.error_if_not_mergeable(&other_token_attributes);

            total_locked_tokens += payment.amount;
        }

        self.send().esdt_local_burn(
            &first_payment.token_identifier,
            first_payment.token_nonce,
            &first_payment.amount,
        );
        self.send().esdt_local_burn_multi(&payments);

        let caller = self.blockchain().get_caller();
        locked_token_mapper.nft_create_and_send(
            &caller,
            total_locked_tokens,
            &first_token_attributes,
        )
    }
}
