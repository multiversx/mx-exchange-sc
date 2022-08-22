elrond_wasm::imports!();
// TODO: Maybe think of some better names?

pub trait PaymentAmountGetter<M: ManagedTypeApi> {
    fn get_payment_amount(&self) -> &BigUint<M>;
}

pub trait RewardPerShareGetter<M: ManagedTypeApi> {
    fn get_reward_per_share(&self) -> &BigUint<M>;
}

pub trait InitialFarmingAmountGetter<M: ManagedTypeApi> {
    fn get_initial_farming_amount(&self) -> &BigUint<M>;
}

pub trait CurrentFarmAmountGetter<M: ManagedTypeApi> {
    fn get_current_farm_amount(&self) -> &BigUint<M>;
}

pub trait CompoundedRewardAmountGetter<M: ManagedTypeApi> {
    fn get_compounded_reward_amount(&self) -> &BigUint<M>;
}
