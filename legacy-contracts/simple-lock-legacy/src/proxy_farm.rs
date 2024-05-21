multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::error_messages::*;

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

#[multiversx_sc::module]
pub trait ProxyFarmModule:
    crate::locked_token::LockedTokenModule + crate::proxy_lp::ProxyLpModule
{
    /// Output payments: the underlying farm tokens
    #[payable("*")]
    #[endpoint(exitFarmLockedToken)]
    fn exit_farm_locked_token(&self) -> EsdtTokenPayment {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        let caller = self.blockchain().get_caller();

        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment);

        let _ = self.check_and_get_unlocked_lp_token(
            &self.lp_proxy_token().get_token_id(),
            farm_proxy_token_attributes.farming_token_locked_nonce,
        );

        self.send().esdt_local_burn(
            &self.lp_proxy_token().get_token_id(),
            farm_proxy_token_attributes.farming_token_locked_nonce,
            &payment.amount,
        );

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        let output_token_payment = EsdtTokenPayment::new(
            farm_proxy_token_attributes.farm_token_id,
            farm_proxy_token_attributes.farm_token_nonce,
            payment.amount,
        );

        self.send().direct_esdt(
            &caller,
            &output_token_payment.token_identifier,
            output_token_payment.token_nonce,
            &output_token_payment.amount,
        );

        output_token_payment
    }

    /// Output payments: the underlying farm tokens
    #[payable("*")]
    #[endpoint(farmClaimRewardsLockedToken)]
    fn farm_claim_rewards_locked_token(&self) -> EsdtTokenPayment {
        self.exit_farm_locked_token()
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

    #[view(getFarmProxyTokenId)]
    #[storage_mapper("farmProxyTokenId")]
    fn farm_proxy_token(&self) -> NonFungibleTokenMapper;
}
