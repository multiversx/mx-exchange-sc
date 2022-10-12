elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::wrapped_lp_attributes::{
    merge_wrapped_lp_tokens_through_factory, WrappedLpToken, WrappedLpTokenAttributes,
};
use fixed_supply_token::FixedSupplyToken;

use super::proxy_common;

#[elrond_wasm::module]
pub trait WrappedLpTokenMerge:
    token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + proxy_common::ProxyCommonModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens_endpoint(&self) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();

        let wrapped_token_mapper = self.wrapped_lp_token();
        let wrapped_lp_tokens = WrappedLpToken::new_from_payments(&payments, &wrapped_token_mapper);

        let merged_tokens = self.merge_wrapped_lp_tokens(wrapped_lp_tokens).payment;
        self.send_payment_non_zero(&caller, &merged_tokens);

        merged_tokens
    }

    fn merge_wrapped_lp_tokens_with_virtual_pos(
        &self,
        wrapped_lp_tokens: ManagedVec<WrappedLpToken<Self::Api>>,
        virtual_pos_attributes: WrappedLpTokenAttributes<Self::Api>,
    ) -> WrappedLpToken<Self::Api> {
        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        let virtual_wrapped_token = WrappedLpToken {
            payment: EsdtTokenPayment::new(
                wrapped_lp_token_id,
                0,
                virtual_pos_attributes.get_total_supply().clone(),
            ),
            attributes: virtual_pos_attributes,
        };

        let mut all_tokens = ManagedVec::from_single_item(virtual_wrapped_token);
        all_tokens.append_vec(wrapped_lp_tokens);

        self.merge_wrapped_lp_tokens(all_tokens)
    }

    fn merge_wrapped_lp_tokens(
        &self,
        wrapped_lp_tokens: ManagedVec<WrappedLpToken<Self::Api>>,
    ) -> WrappedLpToken<Self::Api> {
        let locked_token_id = wrapped_lp_tokens
            .get(0)
            .attributes
            .locked_tokens
            .token_identifier;
        let factory_address = self
            .factory_address_for_locked_token(&locked_token_id)
            .get();

        let new_lp_token_attributes =
            merge_wrapped_lp_tokens_through_factory(factory_address, wrapped_lp_tokens);
        let new_token_amount = new_lp_token_attributes.get_total_supply().clone();
        let new_tokens = self
            .wrapped_lp_token()
            .nft_create(new_token_amount, &new_lp_token_attributes);

        WrappedLpToken {
            payment: new_tokens,
            attributes: new_lp_token_attributes,
        }
    }
}
