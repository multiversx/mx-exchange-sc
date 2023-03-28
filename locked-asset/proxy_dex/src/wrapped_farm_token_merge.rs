multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::proxy_common::INVALID_PAYMENTS_ERR_MSG;
use crate::proxy_common::MIN_MERGE_PAYMENTS;
use crate::wrapped_farm_attributes::merge_wrapped_farm_tokens;
use crate::wrapped_farm_attributes::WrappedFarmToken;
use crate::wrapped_farm_attributes::WrappedFarmTokenAttributes;

use fixed_supply_token::FixedSupplyToken;

#[multiversx_sc::module]
pub trait WrappedFarmTokenMerge:
    token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + crate::sc_whitelist::ScWhitelistModule
    + crate::proxy_common::ProxyCommonModule
    + crate::wrapped_lp_token_merge::WrappedLpTokenMerge
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
{
    #[payable("*")]
    #[endpoint(mergeWrappedFarmTokens)]
    fn merge_wrapped_farm_tokens_endpoint(&self, farm_address: ManagedAddress) -> EsdtTokenPayment {
        self.require_is_intermediated_farm(&farm_address);

        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers();
        require!(
            payments.len() >= MIN_MERGE_PAYMENTS,
            INVALID_PAYMENTS_ERR_MSG
        );

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let wrapped_farm_tokens =
            WrappedFarmToken::new_from_payments(&payments, &wrapped_farm_token_mapper);

        self.send().esdt_local_burn_multi(&payments);

        let merged_tokens = self
            .merge_wrapped_farm_tokens(&caller, farm_address, wrapped_farm_tokens)
            .payment;
        self.send_payment_non_zero(&caller, &merged_tokens);

        merged_tokens
    }

    fn merge_wrapped_farm_tokens_with_virtual_pos(
        &self,
        caller: &ManagedAddress,
        farm_address: ManagedAddress,
        wrapped_farm_tokens: ManagedVec<WrappedFarmToken<Self::Api>>,
        virtual_pos_attributes: WrappedFarmTokenAttributes<Self::Api>,
    ) -> WrappedFarmToken<Self::Api> {
        let wrapped_farm_token_id = self.wrapped_farm_token().get_token_id();
        let virtual_wrapped_token = WrappedFarmToken {
            payment: EsdtTokenPayment::new(
                wrapped_farm_token_id,
                0,
                virtual_pos_attributes.get_total_supply(),
            ),
            attributes: virtual_pos_attributes,
        };

        let mut all_tokens = ManagedVec::from_single_item(virtual_wrapped_token);
        all_tokens.append_vec(wrapped_farm_tokens);

        self.merge_wrapped_farm_tokens(caller, farm_address, all_tokens)
    }

    fn merge_wrapped_farm_tokens(
        &self,
        caller: &ManagedAddress,
        farm_address: ManagedAddress,
        wrapped_farm_tokens: ManagedVec<WrappedFarmToken<Self::Api>>,
    ) -> WrappedFarmToken<Self::Api> {
        let proxy_farming_token = wrapped_farm_tokens.get(0).attributes.proxy_farming_token;
        let locked_token_id = self.get_underlying_locked_token(
            proxy_farming_token.token_identifier,
            proxy_farming_token.token_nonce,
        );

        let factory_address = self.get_factory_address_for_locked_token(&locked_token_id);

        let wrapped_lp_token_mapper = self.wrapped_lp_token();
        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        merge_wrapped_farm_tokens(
            caller,
            factory_address,
            farm_address,
            &wrapped_lp_token_mapper,
            &wrapped_farm_token_mapper,
            wrapped_farm_tokens,
        )
    }
}
