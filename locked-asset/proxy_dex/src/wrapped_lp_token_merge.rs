multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    proxy_common::{INVALID_PAYMENTS_ERR_MSG, MIN_MERGE_PAYMENTS},
    wrapped_lp_attributes::{merge_wrapped_lp_tokens, WrappedLpToken, WrappedLpTokenAttributes},
};
use fixed_supply_token::FixedSupplyToken;

use super::proxy_common;

#[multiversx_sc::module]
pub trait WrappedLpTokenMerge:
    token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + proxy_common::ProxyCommonModule
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
{
    #[payable("*")]
    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens_endpoint(&self) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers();
        require!(
            payments.len() >= MIN_MERGE_PAYMENTS,
            INVALID_PAYMENTS_ERR_MSG
        );

        let wrapped_token_mapper = self.wrapped_lp_token();
        let wrapped_lp_tokens = WrappedLpToken::new_from_payments(&payments, &wrapped_token_mapper);

        self.send().esdt_local_burn_multi(&payments);

        let merged_tokens = self
            .merge_wrapped_lp_tokens(&caller, wrapped_lp_tokens)
            .payment;
        self.send_payment_non_zero(&caller, &merged_tokens);

        merged_tokens
    }

    fn merge_wrapped_lp_tokens_with_virtual_pos(
        &self,
        caller: &ManagedAddress,
        wrapped_lp_tokens: ManagedVec<WrappedLpToken<Self::Api>>,
        virtual_pos_attributes: WrappedLpTokenAttributes<Self::Api>,
    ) -> WrappedLpToken<Self::Api> {
        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        let virtual_wrapped_token = WrappedLpToken {
            payment: EsdtTokenPayment::new(
                wrapped_lp_token_id,
                0,
                virtual_pos_attributes.get_total_supply(),
            ),
            attributes: virtual_pos_attributes,
        };

        let mut all_tokens = ManagedVec::from_single_item(virtual_wrapped_token);
        all_tokens.append_vec(wrapped_lp_tokens);

        self.merge_wrapped_lp_tokens(caller, all_tokens)
    }

    fn merge_wrapped_lp_tokens(
        &self,
        caller: &ManagedAddress,
        wrapped_lp_tokens: ManagedVec<WrappedLpToken<Self::Api>>,
    ) -> WrappedLpToken<Self::Api> {
        let locked_token_id = wrapped_lp_tokens
            .get(0)
            .attributes
            .locked_tokens
            .token_identifier;
        let factory_address = self.get_factory_address_for_locked_token(&locked_token_id);

        let wrapped_lp_token_mapper = self.wrapped_lp_token();
        merge_wrapped_lp_tokens(
            caller,
            factory_address,
            &wrapped_lp_token_mapper,
            wrapped_lp_tokens,
        )
    }
}
