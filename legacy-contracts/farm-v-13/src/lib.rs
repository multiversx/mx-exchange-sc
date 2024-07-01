#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

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
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[multiversx_sc::contract]
pub trait FarmV13 {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        _amount: BigUint,
        _attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[only_owner]
    #[endpoint]
    fn end_produce_rewards(&self) {}
}
