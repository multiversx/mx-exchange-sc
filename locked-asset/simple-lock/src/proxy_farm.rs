multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{error_messages::*, proxy_lp::LpProxyTokenAttributes};

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug, Clone, Copy,
)]
pub enum FarmType {
    SimpleFarm,
    FarmWithLockedRewards,
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

#[multiversx_sc::module]
pub trait ProxyFarmModule:
    crate::farm_interactions::FarmInteractionsModule
    + crate::lp_interactions::LpInteractionsModule
    + crate::locked_token::LockedTokenModule
    + crate::proxy_lp::ProxyLpModule
    + crate::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        let payment_amount = self.call_value().egld_value().clone_value();

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
            self.call_value().all_esdt_transfers().clone_value();
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
        let caller = self.blockchain().get_caller();
        let enter_farm_result = self.call_farm_enter(
            farm_address,
            lp_proxy_token_attributes.lp_token_id.clone(),
            proxy_lp_payment.amount,
            additional_farm_payments,
            caller.clone(),
        );
        let farm_tokens = enter_farm_result.farm_tokens;
        let proxy_farm_token_attributes = FarmProxyTokenAttributes {
            farm_type,
            farm_token_id: farm_tokens.token_identifier,
            farm_token_nonce: farm_tokens.token_nonce,
            farming_token_id: lp_proxy_token_attributes.lp_token_id,
            farming_token_locked_nonce: proxy_lp_payment.token_nonce,
        };

        let farm_tokens = farm_proxy_token_mapper.nft_create_and_send(
            &caller,
            farm_tokens.amount,
            &proxy_farm_token_attributes,
        );

        if enter_farm_result.reward_tokens.amount > 0 {
            self.send().direct_esdt(
                &caller,
                &enter_farm_result.reward_tokens.token_identifier,
                enter_farm_result.reward_tokens.token_nonce,
                &enter_farm_result.reward_tokens.amount,
            );
        }

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
        require!(exit_amount > 0u64, "Exit amount must be greater than 0");
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        require!(
            exit_amount > 0u64 && exit_amount <= payment.amount,
            "Invalid exit amount"
        );
        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment, &exit_amount);

        let farm_address = self.try_get_farm_address(
            &farm_proxy_token_attributes.farming_token_id,
            farm_proxy_token_attributes.farm_type,
        );
        let caller = self.blockchain().get_caller();
        let exit_farm_result = self.call_farm_exit(
            farm_address,
            farm_proxy_token_attributes.farm_token_id,
            farm_proxy_token_attributes.farm_token_nonce,
            payment.amount,
            exit_amount,
            caller.clone(),
        );
        require!(
            exit_farm_result.initial_farming_tokens.token_identifier
                == farm_proxy_token_attributes.farming_token_id,
            INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG
        );

        let lp_proxy_token = self.lp_proxy_token();
        let lp_proxy_token_payment = EsdtTokenPayment::new(
            lp_proxy_token.get_token_id(),
            farm_proxy_token_attributes.farming_token_locked_nonce,
            exit_farm_result.initial_farming_tokens.amount,
        );
        self.send().direct_esdt(
            &caller,
            &lp_proxy_token_payment.token_identifier,
            lp_proxy_token_payment.token_nonce,
            &lp_proxy_token_payment.amount,
        );

        if exit_farm_result.reward_tokens.amount > 0 {
            self.send().direct_esdt(
                &caller,
                &exit_farm_result.reward_tokens.token_identifier,
                exit_farm_result.reward_tokens.token_nonce,
                &exit_farm_result.reward_tokens.amount,
            );
        }

        if exit_farm_result.remaining_farm_tokens.amount > 0 {
            self.send().direct_esdt(
                &caller,
                &payment.token_identifier,
                payment.token_nonce,
                &exit_farm_result.remaining_farm_tokens.amount,
            );
        }

        (lp_proxy_token_payment, exit_farm_result.reward_tokens).into()
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
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment, &payment.amount);

        let farm_address = self.try_get_farm_address(
            &farm_proxy_token_attributes.farming_token_id,
            farm_proxy_token_attributes.farm_type,
        );
        let caller = self.blockchain().get_caller();
        let claim_rewards_result = self.call_farm_claim_rewards(
            farm_address,
            farm_proxy_token_attributes.farm_token_id.clone(),
            farm_proxy_token_attributes.farm_token_nonce,
            payment.amount,
            caller.clone(),
        );
        require!(
            claim_rewards_result.new_farm_tokens.token_identifier
                == farm_proxy_token_attributes.farm_token_id,
            INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG
        );

        farm_proxy_token_attributes.farm_token_nonce =
            claim_rewards_result.new_farm_tokens.token_nonce;

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
        exit_amount: &BigUint,
    ) -> FarmProxyTokenAttributes<Self::Api> {
        require!(payment.amount > 0, NO_PAYMENT_ERR_MSG);

        let farm_proxy_token_mapper = self.farm_proxy_token();
        farm_proxy_token_mapper.require_same_token(&payment.token_identifier);

        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            farm_proxy_token_mapper.get_token_attributes(payment.token_nonce);

        farm_proxy_token_mapper.nft_burn(payment.token_nonce, exit_amount);

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
