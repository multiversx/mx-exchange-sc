multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{FarmToken, FarmTokenAttributes, PaymentsVec};
use fixed_supply_token::FixedSupplyToken;
use math::weighted_average_round_up;
use mergeable::Mergeable;

static NOT_IMPLEMENTED_ERR_MSG: &[u8] = b"Conversion not implemented";

#[derive(
    TypeAbi,
    TopDecode,
    TopEncode,
    NestedDecode,
    NestedEncode,
    ManagedVecItem,
    Clone,
    PartialEq,
    Debug,
)]
pub struct StakingFarmNftTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
    pub original_owner: ManagedAddress<M>,
    pub farming_token_parts: PaymentsVec<M>,
}

#[derive(ManagedVecItem, Clone)]
pub struct StakingFarmToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: StakingFarmNftTokenAttributes<M>,
}

impl<M: ManagedTypeApi> From<FarmTokenAttributes<M>> for StakingFarmNftTokenAttributes<M> {
    fn from(_value: FarmTokenAttributes<M>) -> Self {
        M::error_api_impl().signal_error(NOT_IMPLEMENTED_ERR_MSG);
    }
}

impl<M: ManagedTypeApi> Into<FarmTokenAttributes<M>> for StakingFarmNftTokenAttributes<M> {
    fn into(self) -> FarmTokenAttributes<M> {
        M::error_api_impl().signal_error(NOT_IMPLEMENTED_ERR_MSG);
    }
}

impl<M: ManagedTypeApi> FarmToken<M> for StakingFarmNftTokenAttributes<M> {
    fn get_reward_per_share(&self) -> BigUint<M> {
        self.reward_per_share.clone()
    }

    fn get_compounded_rewards(&self) -> BigUint<M> {
        self.compounded_reward.clone()
    }

    fn get_initial_farming_tokens(&self) -> BigUint<M> {
        &self.current_farm_amount - &self.compounded_reward
    }
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for StakingFarmNftTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.current_farm_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        M::error_api_impl().signal_error(b"Cannot split this token");
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for StakingFarmNftTokenAttributes<M> {
    #[inline]
    fn can_merge_with(&self, other: &Self) -> bool {
        self.original_owner == other.original_owner
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let first_supply = self.get_total_supply();
        let second_supply = other.get_total_supply();
        self.reward_per_share = weighted_average_round_up(
            self.reward_per_share.clone(),
            first_supply,
            other.reward_per_share.clone(),
            second_supply,
        );

        self.compounded_reward += other.compounded_reward;
        self.current_farm_amount += other.current_farm_amount;
        self.farming_token_parts
            .append_vec(other.farming_token_parts);
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct UnbondSftAttributes/*<M: ManagedTypeApi>*/ {
    pub unlock_epoch: u64,
    // pub farming_token_parts: PaymentsVec<M>,
}
