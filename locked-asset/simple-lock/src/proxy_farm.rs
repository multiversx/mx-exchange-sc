elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    error_messages::*,
    farm_interactions::{ExitFarmResult, ExitFarmResultWrapper},
    proxy_lp::LpProxyTokenAttributes,
};

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug, Clone, Copy,
)]
pub enum FarmType {
    SimpleFarm,
    FarmWithLockedRewards,
    FarmWithBoostedRewards,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct FarmProxyTokenAttributes<M: ManagedTypeApi> {
    pub farm_type: FarmType,
    pub farm_token_id: TokenIdentifier<M>,
    pub farm_token_nonce: u64,
    pub farming_token_id: TokenIdentifier<M>,
    pub farming_token_locked_nonce: u64,
}

pub type EnterFarmThroughProxyResultType<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type ExitFarmThroughProxyResultType<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type FarmClaimRewardsThroughProxyResultType<M> =
    MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type FarmCompoundRewardsThroughProxyResultType<M> = EsdtTokenPayment<M>;

#[elrond_wasm::module]
pub trait ProxyFarmModule:
    crate::farm_interactions::FarmInteractionsModule
    + crate::lp_interactions::LpInteractionsModule
    + crate::locked_token::LockedTokenModule
    + crate::proxy_lp::ProxyLpModule
    + crate::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueFarmProxyToken)]
    fn issue_farm_proxy_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value();

        self.farm_proxy_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    /// Add a farm to the whitelist.
    /// Currently, two types of farms are supported, denoted by the `farm_type` argument:
    /// `0` - SimpleFarm - rewards are fungible tokens
    /// `1` - FarmWithLockedRewards - rewards are META ESDT locked tokens
    #[only_owner]
    #[endpoint(addFarmToWhitelist)]
    fn add_farm_to_whitelist(
        &self,
        farm_address: ManagedAddress,
        farming_token_id: TokenIdentifier,
        farm_type: FarmType,
    ) {
        require!(
            self.blockchain().is_smart_contract(&farm_address),
            INVALID_SC_ADDRESS_ERR_MSG
        );

        self.farm_address_for_token(&farming_token_id, farm_type)
            .set(&farm_address);

        let is_new_farm = self.known_farms().insert(farm_address);
        require!(is_new_farm, "Farm address already known");
    }

    #[only_owner]
    #[endpoint(removeFarmFromWhitelist)]
    fn remove_farm_from_whitelist(
        &self,
        farm_address: ManagedAddress,
        farming_token_id: TokenIdentifier,
        farm_type: FarmType,
    ) {
        let was_removed = self.known_farms().swap_remove(&farm_address);
        require!(was_removed, "Farm address not known");

        let stored_addr = self
            .farm_address_for_token(&farming_token_id, farm_type)
            .take();
        require!(
            stored_addr == farm_address,
            "Farm address does not match the given token and farm type"
        );
    }

    /// Enter farm with LOCKED tokens.
    /// User will choose if they want to enter a farm with normal rewards, or locked rewards.
    ///
    /// Expected payment: LOCKED LP tokens (gained from add_liquidity_locked_token)
    ///
    /// Arguments:
    /// - farm_type - The farm type the user wishes to enter (unlocked or locked rewards)
    ///
    /// Output payments:
    /// - FARM_PROXY token, which can later be used to further interact with the specific farm
    #[payable("*")]
    #[endpoint(enterFarmLockedToken)]
    fn enter_farm_locked_token(
        &self,
        farm_type: FarmType,
    ) -> EnterFarmThroughProxyResultType<Self::Api> {
        let payments: ManagedVec<EsdtTokenPayment<Self::Api>> =
            self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), NO_PAYMENT_ERR_MSG);

        let proxy_lp_payment: EsdtTokenPayment<Self::Api> = payments.get(0);
        let lp_proxy_token_mapper = self.lp_proxy_token();
        lp_proxy_token_mapper.require_same_token(&proxy_lp_payment.token_identifier);

        let lp_proxy_token_attributes: LpProxyTokenAttributes<Self::Api> =
            lp_proxy_token_mapper.get_token_attributes(proxy_lp_payment.token_nonce);

        let farm_proxy_token_mapper = self.farm_proxy_token();
        let additional_proxy_farm_tokens = payments.slice(1, payments.len()).unwrap_or_default();
        let mut additional_farm_payments = ManagedVec::new();
        for p in &additional_proxy_farm_tokens {
            let proxy_farm_attributes: FarmProxyTokenAttributes<Self::Api> =
                farm_proxy_token_mapper.get_token_attributes(p.token_nonce);

            let same_farming_token =
                proxy_farm_attributes.farming_token_id == lp_proxy_token_attributes.lp_token_id;
            let same_farming_nonce =
                proxy_farm_attributes.farming_token_locked_nonce == proxy_lp_payment.token_nonce;
            let same_farm_type = proxy_farm_attributes.farm_type == farm_type;
            require!(
                same_farming_token && same_farming_nonce && same_farm_type,
                INVALID_PAYMENTS_ERR_MSG
            );

            farm_proxy_token_mapper.nft_burn(p.token_nonce, &p.amount);

            additional_farm_payments.push(EsdtTokenPayment::new(
                proxy_farm_attributes.farm_token_id,
                proxy_farm_attributes.farm_token_nonce,
                p.amount,
            ));
        }

        let farm_address =
            self.try_get_farm_address(&lp_proxy_token_attributes.lp_token_id, farm_type);
        let enter_farm_result = self.call_farm_enter(
            farm_address,
            lp_proxy_token_attributes.lp_token_id.clone(),
            proxy_lp_payment.amount,
            additional_farm_payments,
        );
        let farm_tokens = enter_farm_result.farm_tokens;
        let proxy_farm_token_attributes = FarmProxyTokenAttributes {
            farm_type,
            farm_token_id: farm_tokens.token_identifier,
            farm_token_nonce: farm_tokens.token_nonce,
            farming_token_id: lp_proxy_token_attributes.lp_token_id,
            farming_token_locked_nonce: proxy_lp_payment.token_nonce,
        };

        let caller = self.blockchain().get_caller();
        let farm_tokens = farm_proxy_token_mapper.nft_create_and_send(
            &caller,
            farm_tokens.amount,
            &proxy_farm_token_attributes,
        );

        self.send()
            .direct_non_zero_esdt_payment(&caller, &enter_farm_result.reward_tokens);

        (farm_tokens, enter_farm_result.reward_tokens).into()
    }

    /// Exit a farm previously entered through `enterFarmLockedToken`.
    ///
    /// Expected payment: FARM_PROXY tokens
    ///
    /// Output Payments:
    /// - original farming tokens
    /// - farm reward tokens
    #[payable("*")]
    #[endpoint(exitFarmLockedToken)]
    fn exit_farm_locked_token(
        &self,
        exit_amount: BigUint,
    ) -> ExitFarmThroughProxyResultType<Self::Api> {
        self.exit_farm_base_impl::<ExitFarmResultWrapper<Self::Api>>(OptionalValue::Some(
            exit_amount,
        ))
    }

    fn exit_farm_base_impl<ResultsType: ExitFarmResult<Self::Api>>(
        &self,
        opt_exit_amount: OptionalValue<BigUint>,
    ) -> ExitFarmThroughProxyResultType<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment);

        let farm_address = self.try_get_farm_address(
            &farm_proxy_token_attributes.farming_token_id,
            farm_proxy_token_attributes.farm_type,
        );
        let exit_farm_result = self.call_farm_exit::<ResultsType>(
            farm_address,
            farm_proxy_token_attributes.farm_token_id,
            farm_proxy_token_attributes.farm_token_nonce,
            payment.amount,
            opt_exit_amount,
        );

        let initial_farming_tokens = exit_farm_result.get_initial_farming_tokens();
        require!(
            initial_farming_tokens.token_identifier == farm_proxy_token_attributes.farming_token_id,
            INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG
        );

        let lp_proxy_token = self.lp_proxy_token();
        let lp_proxy_token_payment = EsdtTokenPayment::new(
            lp_proxy_token.get_token_id(),
            farm_proxy_token_attributes.farming_token_locked_nonce,
            initial_farming_tokens.amount,
        );

        let mut output_payments = ManagedVec::new();
        if lp_proxy_token_payment.amount > 0 {
            output_payments.push(lp_proxy_token_payment.clone());
        }

        let reward_tokens = exit_farm_result.get_reward_tokens();
        if reward_tokens.amount > 0 {
            output_payments.push(reward_tokens.clone());
        }

        let remaining_farm_tokens = exit_farm_result.get_remaining_farm_tokens();
        if remaining_farm_tokens.amount > 0 {
            let farm_tokens_payment = EsdtTokenPayment::new(
                payment.token_identifier,
                payment.token_nonce,
                remaining_farm_tokens.amount,
            );
            output_payments.push(farm_tokens_payment);
        }

        if !output_payments.is_empty() {
            let caller = self.blockchain().get_caller();
            self.send().direct_multi(&caller, &output_payments);
        }

        (lp_proxy_token_payment, reward_tokens).into()
    }

    /// Claim rewards from a previously entered farm.
    /// The FARM_PROXY tokens are burned, and new ones are created.
    /// This is needed because every farm action changes the farm token nonce
    ///
    /// Expected payment: FARM_PROXY tokens
    ///
    /// Output payments:
    /// - a new FARM_PROXY token
    /// - farm reward tokens
    #[payable("*")]
    #[endpoint(farmClaimRewardsLockedToken)]
    fn farm_claim_rewards_locked_token(&self) -> FarmClaimRewardsThroughProxyResultType<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        let mut farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment);

        let farm_address = self.try_get_farm_address(
            &farm_proxy_token_attributes.farming_token_id,
            farm_proxy_token_attributes.farm_type,
        );
        let claim_rewards_result = self.call_farm_claim_rewards(
            farm_address,
            farm_proxy_token_attributes.farm_token_id.clone(),
            farm_proxy_token_attributes.farm_token_nonce,
            payment.amount,
        );
        require!(
            claim_rewards_result.new_farm_tokens.token_identifier
                == farm_proxy_token_attributes.farm_token_id,
            INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG
        );

        farm_proxy_token_attributes.farm_token_nonce =
            claim_rewards_result.new_farm_tokens.token_nonce;

        let caller = self.blockchain().get_caller();
        let new_proxy_token_payment = self.farm_proxy_token().nft_create_and_send(
            &caller,
            claim_rewards_result.new_farm_tokens.amount,
            &farm_proxy_token_attributes,
        );

        if claim_rewards_result.reward_tokens.amount > 0 {
            self.send().direct_esdt(
                &caller,
                &claim_rewards_result.reward_tokens.token_identifier,
                claim_rewards_result.reward_tokens.token_nonce,
                &claim_rewards_result.reward_tokens.amount,
            );
        }

        (new_proxy_token_payment, claim_rewards_result.reward_tokens).into()
    }

    fn try_get_farm_address(
        &self,
        farming_token_id: &TokenIdentifier,
        farm_type: FarmType,
    ) -> ManagedAddress {
        let mapper = self.farm_address_for_token(farming_token_id, farm_type);
        require!(
            !mapper.is_empty(),
            "No farm address for the specified token and type pair",
        );

        mapper.get()
    }

    fn validate_payment_and_get_farm_proxy_token_attributes(
        &self,
        payment: &EsdtTokenPayment<Self::Api>,
    ) -> FarmProxyTokenAttributes<Self::Api> {
        require!(payment.amount > 0, NO_PAYMENT_ERR_MSG);

        let farm_proxy_token_mapper = self.farm_proxy_token();
        farm_proxy_token_mapper.require_same_token(&payment.token_identifier);

        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            farm_proxy_token_mapper.get_token_attributes(payment.token_nonce);

        farm_proxy_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        farm_proxy_token_attributes
    }

    #[view(getKnownFarms)]
    #[storage_mapper("knownFarms")]
    fn known_farms(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("farmAddressForToken")]
    fn farm_address_for_token(
        &self,
        farming_token_id: &TokenIdentifier,
        farm_type: FarmType,
    ) -> SingleValueMapper<ManagedAddress>;

    #[view(getFarmProxyTokenId)]
    #[storage_mapper("farmProxyTokenId")]
    fn farm_proxy_token(&self) -> NonFungibleTokenMapper;
}
