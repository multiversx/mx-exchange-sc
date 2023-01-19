#![allow(clippy::too_many_arguments)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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

#[multiversx_sc::module]
pub trait ProxyFarmModule:
    crate::proxy_common::ProxyCommonModule
    + crate::sc_whitelist::ScWhitelistModule
    + crate::proxy_pair::ProxyPairModule
    + crate::pair_interactions::PairInteractionsModule
    + crate::farm_interactions::FarmInteractionsModule
    + crate::energy_update::EnergyUpdateModule
    + energy_query::EnergyQueryModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + crate::wrapped_farm_token_merge::WrappedFarmTokenMerge
    + crate::wrapped_lp_token_merge::WrappedLpTokenMerge
    + crate::events::EventsModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
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
        let farm_farming_token_pair = if self.is_locked_token(&proxy_farming_token.token_identifier)
        {
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

            self.send().esdt_local_burn_multi(&payments);

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
        let asset_token_id = self.get_base_token_id();
        self.send()
            .esdt_local_mint(&asset_token_id, 0, &locked_token.amount);

        let minted_asset_tokens = EsdtTokenPayment::new(asset_token_id, 0, locked_token.amount);
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
        farm_address: ManagedAddress,
        exit_amount: BigUint,
    ) -> ExitFarmProxyResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let payment = self.call_value().single_esdt();
        wrapped_farm_token_mapper.require_same_token(&payment.token_identifier);

        let full_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api> = self
            .blockchain()
            .get_token_attributes(&payment.token_identifier, payment.token_nonce);

        let wrapped_farm_attributes_for_exit: WrappedFarmTokenAttributes<Self::Api> =
            full_wrapped_farm_attributes
                .clone()
                .into_part(&payment.amount);
        let exit_result = self.call_exit_farm(
            farm_address.clone(),
            wrapped_farm_attributes_for_exit.farm_token,
            exit_amount.clone(),
        );

        let mut remaining_wrapped_tokens = payment.clone();
        remaining_wrapped_tokens.amount = exit_result.remaining_farm_tokens.amount;

        self.burn_if_base_asset(&exit_result.farming_tokens);

        let mut payment_used_for_exit = payment.clone();
        payment_used_for_exit.amount = exit_amount.clone();
        let wrapped_attributes_used_for_exit_farm: WrappedFarmTokenAttributes<Self::Api> =
            full_wrapped_farm_attributes.into_part(&exit_amount);
        let wrapped_farm_tokens_for_initial_tokens = WrappedFarmToken {
            payment: payment_used_for_exit,
            attributes: wrapped_attributes_used_for_exit_farm.clone(),
        };

        let caller = self.blockchain().get_caller();
        let initial_proxy_farming_tokens = self
            .handle_farm_penalty_and_get_output_proxy_farming_token(
                &caller,
                wrapped_farm_tokens_for_initial_tokens,
                exit_result.farming_tokens.amount,
            );

        self.send_payment_non_zero(&caller, &initial_proxy_farming_tokens);
        self.send_payment_non_zero(&caller, &exit_result.reward_tokens);
        self.send_payment_non_zero(&caller, &remaining_wrapped_tokens);

        wrapped_farm_token_mapper.nft_burn(payment.token_nonce, &exit_amount);

        self.emit_exit_farm_proxy_event(
            &caller,
            &farm_address,
            payment,
            wrapped_attributes_used_for_exit_farm,
            exit_result.reward_tokens.clone(),
        );

        (
            initial_proxy_farming_tokens,
            exit_result.reward_tokens,
            remaining_wrapped_tokens,
        )
            .into()
    }

    fn handle_farm_penalty_and_get_output_proxy_farming_token(
        &self,
        caller: &ManagedAddress,
        wrapped_farm_tokens: WrappedFarmToken<Self::Api>,
        farming_tokens_amount_from_farm: BigUint,
    ) -> EsdtTokenPayment {
        require!(
            wrapped_farm_tokens.payment.amount >= farming_tokens_amount_from_farm,
            "Invalid payments received from Farm"
        );

        if wrapped_farm_tokens.payment.amount == farming_tokens_amount_from_farm {
            return wrapped_farm_tokens.attributes.proxy_farming_token;
        }

        let penalty_amount = &wrapped_farm_tokens.payment.amount - &farming_tokens_amount_from_farm;
        let proxy_farming_token = &wrapped_farm_tokens.attributes.proxy_farming_token;
        let mut remaining_proxy_tokens = proxy_farming_token.clone();
        remaining_proxy_tokens.amount -= &penalty_amount;

        if self.is_locked_token(&proxy_farming_token.token_identifier) {
            self.burn_locked_tokens_and_update_energy(
                &proxy_farming_token.token_identifier,
                proxy_farming_token.token_nonce,
                &penalty_amount,
                caller,
            );

            return remaining_proxy_tokens;
        }

        let wrapped_lp_tokens_mapper = self.wrapped_lp_token();
        let old_wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api> = self
            .get_attributes_as_part_of_fixed_supply(proxy_farming_token, &wrapped_lp_tokens_mapper);
        let new_wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api> = self
            .get_attributes_as_part_of_fixed_supply(
                &remaining_proxy_tokens,
                &wrapped_lp_tokens_mapper,
            );
        let extra_locked_tokens = &old_wrapped_lp_attributes.locked_tokens.amount
            - &new_wrapped_lp_attributes.locked_tokens.amount;
        self.burn_locked_tokens_and_update_energy(
            &new_wrapped_lp_attributes.locked_tokens.token_identifier,
            new_wrapped_lp_attributes.locked_tokens.token_nonce,
            &extra_locked_tokens,
            caller,
        );

        wrapped_lp_tokens_mapper
            .nft_create(remaining_proxy_tokens.amount, &new_wrapped_lp_attributes)
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

    fn require_wrapped_farm_token_id_not_empty(&self) {
        require!(!self.wrapped_farm_token().is_empty(), "Empty token id");
    }
}
