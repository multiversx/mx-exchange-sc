multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;

// lp farm

pub struct LpFarmClaimRewardsResult<M: ManagedTypeApi> {
    pub new_lp_farm_tokens: EsdtTokenPayment<M>,
    pub lp_farm_rewards: EsdtTokenPayment<M>,
}

pub struct LpFarmExitResult<M: ManagedTypeApi> {
    pub lp_tokens: EsdtTokenPayment<M>,
    pub lp_farm_rewards: EsdtTokenPayment<M>,
}

// staking farm

pub struct StakingFarmEnterResult<M: ManagedTypeApi> {
    pub received_staking_farm_token: EsdtTokenPayment<M>,
    pub boosted_rewards: EsdtTokenPayment<M>,
}

pub struct StakingFarmClaimRewardsResult<M: ManagedTypeApi> {
    pub new_staking_farm_tokens: EsdtTokenPayment<M>,
    pub staking_farm_rewards: EsdtTokenPayment<M>,
}

pub struct StakingFarmExitResult<M: ManagedTypeApi> {
    pub unbond_staking_farm_token: EsdtTokenPayment<M>,
    pub staking_rewards: EsdtTokenPayment<M>,
}

// pair

pub struct PairRemoveLiquidityResult<M: ManagedTypeApi> {
    pub staking_token_payment: EsdtTokenPayment<M>,
    pub other_token_payment: EsdtTokenPayment<M>,
}

// proxy return types

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct StakeProxyResult<M: ManagedTypeApi> {
    pub dual_yield_tokens: EsdtTokenPayment<M>,
    pub staking_boosted_rewards: EsdtTokenPayment<M>,
    pub lp_farm_boosted_rewards: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> StakeProxyResult<M> {
    pub fn send_and_return<SC: token_send::TokenSendModule<Api = M>>(
        self,
        sc: &SC,
        to: &ManagedAddress<M>,
    ) -> Self {
        sc.send_payment_non_zero(to, &self.dual_yield_tokens);
        sc.send_payment_non_zero(to, &self.staking_boosted_rewards);
        sc.send_payment_non_zero(to, &self.lp_farm_boosted_rewards);

        self
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct ClaimDualYieldResult<M: ManagedTypeApi> {
    pub lp_farm_rewards: EsdtTokenPayment<M>,
    pub staking_farm_rewards: EsdtTokenPayment<M>,
    pub new_dual_yield_tokens: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> ClaimDualYieldResult<M> {
    pub fn send_and_return<SC: token_send::TokenSendModule<Api = M>>(
        self,
        sc: &SC,
        to: &ManagedAddress<M>,
    ) -> Self {
        let mut payments = ManagedVec::new();
        payments.push(self.lp_farm_rewards.clone());
        payments.push(self.staking_farm_rewards.clone());
        payments.push(self.new_dual_yield_tokens.clone());

        sc.send_multiple_tokens_if_not_zero(to, &payments);

        self
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct UnstakeResult<M: ManagedTypeApi> {
    pub other_token_payment: EsdtTokenPayment<M>,
    pub lp_farm_rewards: EsdtTokenPayment<M>,
    pub staking_rewards: EsdtTokenPayment<M>,
    pub unbond_staking_farm_token: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> UnstakeResult<M> {
    pub fn send_and_return<SC: token_send::TokenSendModule<Api = M>>(
        self,
        sc: &SC,
        to: &ManagedAddress<M>,
    ) -> Self {
        let mut payments = ManagedVec::new();
        payments.push(self.other_token_payment.clone());
        payments.push(self.lp_farm_rewards.clone());
        payments.push(self.staking_rewards.clone());
        payments.push(self.unbond_staking_farm_token.clone());

        sc.send_multiple_tokens_if_not_zero(to, &payments);

        self
    }
}

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct MergeResult<M: ManagedTypeApi> {
    pub lp_farm_rewards: EsdtTokenPayment<M>,
    pub staking_farm_rewards: EsdtTokenPayment<M>,
    pub new_dual_yield_tokens: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> MergeResult<M> {
    pub fn send_and_return<SC: token_send::TokenSendModule<Api = M>>(
        self,
        sc: &SC,
        to: &ManagedAddress<M>,
    ) -> Self {
        let mut payments = ManagedVec::new();
        payments.push(self.lp_farm_rewards.clone());
        payments.push(self.staking_farm_rewards.clone());
        payments.push(self.new_dual_yield_tokens.clone());

        sc.send_multiple_tokens_if_not_zero(to, &payments);

        self
    }
}
