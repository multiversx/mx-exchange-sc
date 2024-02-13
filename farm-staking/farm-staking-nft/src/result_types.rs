multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct MergeResultType<M: ManagedTypeApi> {
    pub merged_farm_token: EsdtTokenPayment<M>,
    pub boosted_rewards_payment: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct EnterFarmResultType<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
    pub boosted_rewards_payment: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct ClaimRewardsResultType<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
    pub rewards: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct CompoundRewardsResultType<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct UnstakeRewardsResultType<M: ManagedTypeApi> {
    pub unbond_farm_token: EsdtTokenPayment<M>,
    pub reward_payment: EsdtTokenPayment<M>,
}
