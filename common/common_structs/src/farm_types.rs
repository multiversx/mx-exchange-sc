elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    CompoundedRewardAmountGetter, CurrentFarmAmountGetter, Epoch, InitialFarmingAmountGetter,
    RewardPerShareGetter, Energy,
};

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_user: ManagedAddress<M>,
    pub original_entering_epoch: Epoch,
    pub entering_epoch: Epoch,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
    pub energy: Energy<M>,
}

impl<M: ManagedTypeApi> RewardPerShareGetter<M> for FarmTokenAttributes<M> {
    fn get_reward_per_share(&self) -> &BigUint<M> {
        &self.reward_per_share
    }
}

impl<M: ManagedTypeApi> InitialFarmingAmountGetter<M> for FarmTokenAttributes<M> {
    fn get_initial_farming_amount(&self) -> &BigUint<M> {
        &self.initial_farming_amount
    }
}

impl<M: ManagedTypeApi> CurrentFarmAmountGetter<M> for FarmTokenAttributes<M> {
    fn get_current_farm_amount(&self) -> &BigUint<M> {
        &self.current_farm_amount
    }
}

impl<M: ManagedTypeApi> CompoundedRewardAmountGetter<M> for FarmTokenAttributes<M> {
    fn get_compounded_reward_amount(&self) -> &BigUint<M> {
        &self.compounded_reward
    }
}
