elrond_wasm::imports!();

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
