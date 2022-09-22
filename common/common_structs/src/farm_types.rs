elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::Epoch;

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
    pub original_entering_epoch: Epoch,
    pub entering_epoch: Epoch,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}
