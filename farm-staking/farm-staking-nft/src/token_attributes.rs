multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::PaymentsVec;
use math::weighted_average_round_up;
use mergeable::Mergeable;

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
    pub original_owner: ManagedAddress<M>,
    pub farming_token_parts: PaymentsVec<M>,
}

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
pub struct PartialStakingFarmNftTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub original_owner: ManagedAddress<M>,
    pub farming_token_parts: PaymentsVec<M>,
    pub current_farm_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> PartialStakingFarmNftTokenAttributes<M> {
    pub fn into_full(self) -> StakingFarmNftTokenAttributes<M> {
        StakingFarmNftTokenAttributes {
            reward_per_share: self.reward_per_share,
            compounded_reward: self.compounded_reward,
            original_owner: self.original_owner,
            farming_token_parts: self.farming_token_parts,
        }
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for PartialStakingFarmNftTokenAttributes<M> {
    #[inline]
    fn can_merge_with(&self, other: &Self) -> bool {
        self.original_owner == other.original_owner
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let first_supply = self.current_farm_amount.clone();
        let second_supply = other.current_farm_amount.clone();
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
pub struct UnbondSftAttributes /*<M: ManagedTypeApi>*/ {
    pub unlock_epoch: u64,
    // pub farming_token_parts: PaymentsVec<M>,
}

#[derive(ManagedVecItem, Clone)]
pub struct StakingFarmToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: StakingFarmNftTokenAttributes<M>,
}
