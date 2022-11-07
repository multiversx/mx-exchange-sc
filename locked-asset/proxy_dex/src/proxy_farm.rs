#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use fixed_supply_token::FixedSupplyToken;

use crate::{
    proxy_common::INVALID_PAYMENTS_ERR_MSG,
    wrapped_farm_attributes::{WrappedFarmToken, WrappedFarmTokenAttributes},
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

pub struct FarmingFarmTokenPair<M: ManagedTypeApi> {
    pub farming_token: EsdtTokenPayment<M>,
    pub farm_token: EsdtTokenPayment<M>,
}

pub type ExitFarmProxyResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type ClaimRewardsFarmProxyResultType<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;

#[elrond_wasm::module]
pub trait ProxyFarmModule:
    crate::proxy_common::ProxyCommonModule
    + crate::sc_whitelist::ScWhitelistModule
    + crate::proxy_pair::ProxyPairModule
    + crate::pair_interactions::PairInteractionsModule
    + crate::farm_interactions::FarmInteractionsModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + crate::wrapped_farm_token_merge::WrappedFarmTokenMerge
    + crate::wrapped_lp_token_merge::WrappedLpTokenMerge
    + crate::events::EventsModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(enterFarmProxy)]
    fn enter_farm_proxy_endpoint(&self, farm_address: ManagedAddress) -> EsdtTokenPayment {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let caller = self.blockchain().get_caller();
        let mut payments = self.get_non_empty_payments();
        let proxy_farming_token = self.pop_first_payment(&mut payments);

        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        let is_input_locked_token = self
            .locked_token_ids()
            .contains(&proxy_farming_token.token_identifier);

        let farm_farming_token_pair = if is_input_locked_token {
            self.enter_farm_locked_token(farm_address.clone(), proxy_farming_token.clone())
        } else if proxy_farming_token.token_identifier == wrapped_lp_token_id {
            self.enter_farm_wrapped_lp(farm_address.clone(), proxy_farming_token.clone())
        } else {
            sc_panic!(INVALID_PAYMENTS_ERR_MSG)
        };

        let new_token_attributes = WrappedFarmTokenAttributes {
            farm_token: farm_farming_token_pair.farm_token,
            proxy_farming_token,
        };

        let wrapped_farm_mapper = self.wrapped_farm_token();
        let token_merge_requested = !payments.is_empty();
        let new_wrapped_farm_token = if token_merge_requested {
            let wrapped_lp_tokens =
                WrappedFarmToken::new_from_payments(&payments, &wrapped_farm_mapper);

            self.burn_multi_esdt(&payments);

            self.merge_wrapped_farm_tokens_with_virtual_pos(
                &caller,
                farm_address.clone(),
                wrapped_lp_tokens,
                new_token_attributes,
            )
        } else {
            let new_token_amount = new_token_attributes.get_total_supply();
            let output_wrapped_farm_token =
                wrapped_farm_mapper.nft_create(new_token_amount, &new_token_attributes);

            WrappedFarmToken {
                payment: output_wrapped_farm_token,
                attributes: new_token_attributes,
            }
        };

        self.send_payment_non_zero(&caller, &new_wrapped_farm_token.payment);

        self.emit_enter_farm_proxy_event(
            &caller,
            &farm_address,
            farm_farming_token_pair.farming_token,
            new_wrapped_farm_token.payment.clone(),
            new_wrapped_farm_token.attributes,
            token_merge_requested,
        );

        new_wrapped_farm_token.payment
    }

    fn enter_farm_locked_token(
        &self,
        farm_address: ManagedAddress,
        locked_token: EsdtTokenPayment,
    ) -> FarmingFarmTokenPair<Self::Api> {
        let minted_asset_tokens = self.asset_token().mint(locked_token.amount);
        let enter_result = self.call_enter_farm(
            farm_address,
            minted_asset_tokens.token_identifier.clone(),
            minted_asset_tokens.amount.clone(),
        );

        FarmingFarmTokenPair {
            farming_token: minted_asset_tokens,
            farm_token: enter_result.farm_token,
        }
    }

    fn enter_farm_wrapped_lp(
        &self,
        farm_address: ManagedAddress,
        wrapped_lp_token: EsdtTokenPayment,
    ) -> FarmingFarmTokenPair<Self::Api> {
        let wrapped_lp_token_mapper = self.wrapped_lp_token();
        let wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api> = self
            .get_attributes_as_part_of_fixed_supply(&wrapped_lp_token, &wrapped_lp_token_mapper);

        let farming_token = EsdtTokenPayment::new(
            wrapped_lp_attributes.lp_token_id,
            0,
            wrapped_lp_attributes.lp_token_amount,
        );
        let enter_result = self.call_enter_farm(
            farm_address,
            farming_token.token_identifier.clone(),
            farming_token.amount.clone(),
        );

        FarmingFarmTokenPair {
            farming_token,
            farm_token: enter_result.farm_token,
        }
    }

    #[payable("*")]
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(
        &self,
        exit_amount: BigUint,
        farm_address: ManagedAddress,
    ) -> ExitFarmProxyResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let payment = self.call_value().single_esdt();
        wrapped_farm_token_mapper.require_same_token(&payment.token_identifier);

        let wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_farm_token_mapper);
        let exit_result = self.call_exit_farm(
            farm_address.clone(),
            wrapped_farm_attributes.farm_token.clone(),
            exit_amount,
        );

        wrapped_farm_token_mapper.nft_burn(payment.token_nonce, &payment.amount);
        self.burn_if_base_asset(&exit_result.farming_tokens);

        let initial_proxy_farming_tokens = wrapped_farm_attributes.proxy_farming_token.clone();
        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &initial_proxy_farming_tokens);
        self.send_payment_non_zero(&caller, &exit_result.reward_tokens);
        self.send_payment_non_zero(&caller, &exit_result.remaining_farm_tokens);

        self.emit_exit_farm_proxy_event(
            &caller,
            &farm_address,
            payment,
            wrapped_farm_attributes,
            exit_result.reward_tokens.clone(),
        );

        (
            initial_proxy_farming_tokens,
            exit_result.reward_tokens,
            exit_result.remaining_farm_tokens,
        )
            .into()
    }

    #[payable("*")]
    #[endpoint(claimRewardsProxy)]
    fn claim_rewards_proxy(
        &self,
        farm_address: ManagedAddress,
    ) -> ClaimRewardsFarmProxyResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let payment = self.call_value().single_esdt();
        wrapped_farm_token_mapper.require_same_token(&payment.token_identifier);

        let wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_farm_token_mapper);
        let claim_result = self.call_claim_rewards_farm(
            farm_address.clone(),
            wrapped_farm_attributes.farm_token.clone(),
        );

        let new_wrapped_farm_attributes = WrappedFarmTokenAttributes {
            farm_token: claim_result.new_farm_token,
            proxy_farming_token: wrapped_farm_attributes.proxy_farming_token.clone(),
        };
        let new_token_amount = new_wrapped_farm_attributes.get_total_supply();
        let new_wrapped_token =
            wrapped_farm_token_mapper.nft_create(new_token_amount, &new_wrapped_farm_attributes);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &new_wrapped_token);
        self.send_payment_non_zero(&caller, &claim_result.rewards);

        self.emit_claim_rewards_farm_proxy_event(
            &caller,
            &farm_address,
            payment,
            wrapped_farm_attributes,
            new_wrapped_token.clone(),
            new_wrapped_farm_attributes,
            claim_result.rewards.clone(),
        );

        (new_wrapped_token, claim_result.rewards).into()
    }

    #[payable("*")]
    #[endpoint(compoundRewardsProxy)]
    fn compound_rewards_proxy(&self, farm_address: ManagedAddress) -> EsdtTokenPayment {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let payment = self.call_value().single_esdt();
        wrapped_farm_token_mapper.require_same_token(&payment.token_identifier);

        let wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_farm_token_mapper);
        let comp_result = self.call_compound_rewards_farm(
            farm_address.clone(),
            wrapped_farm_attributes.farm_token.clone(),
        );

        wrapped_farm_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let new_wrapped_farm_attributes = WrappedFarmTokenAttributes {
            farm_token: comp_result.new_farm_token,
            proxy_farming_token: wrapped_farm_attributes.proxy_farming_token.clone(),
        };
        let new_token_amount = new_wrapped_farm_attributes.get_total_supply();
        let new_wrapped_token =
            wrapped_farm_token_mapper.nft_create(new_token_amount, &new_wrapped_farm_attributes);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &new_wrapped_token);

        self.emit_compound_rewards_farm_proxy_event(
            &caller,
            &farm_address,
            payment,
            wrapped_farm_attributes,
            new_wrapped_token.clone(),
            new_wrapped_farm_attributes,
        );

        new_wrapped_token
    }

    fn require_wrapped_farm_token_id_not_empty(&self) {
        require!(!self.wrapped_farm_token().is_empty(), "Empty token id");
    }
}
