multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use fixed_supply_token::FixedSupplyToken;
use math::weighted_average_round_up;
use mergeable::Mergeable;

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
    pub entering_epoch: Epoch,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
    pub original_owner: ManagedAddress<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for FarmTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.current_farm_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_compounded_reward = self.rule_of_three(payment_amount, &self.compounded_reward);
        let new_current_farm_amount = payment_amount.clone();

        FarmTokenAttributes {
            reward_per_share: self.reward_per_share,
            entering_epoch: self.entering_epoch,
            compounded_reward: new_compounded_reward,
            current_farm_amount: new_current_farm_amount,
            original_owner: self.original_owner,
        }
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for FarmTokenAttributes<M> {
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

        self.entering_epoch = core::cmp::max(self.entering_epoch, other.entering_epoch);
    }
}

pub trait FarmToken<M: ManagedTypeApi> {
    fn get_reward_per_share(&self) -> BigUint<M>;

    fn get_compounded_rewards(&self) -> BigUint<M>;

    fn get_initial_farming_tokens(&self) -> BigUint<M>;
}

impl<M: ManagedTypeApi> FarmToken<M> for FarmTokenAttributes<M> {
    #[inline]
    fn get_reward_per_share(&self) -> BigUint<M> {
        self.reward_per_share.clone()
    }

    #[inline]
    fn get_compounded_rewards(&self) -> BigUint<M> {
        self.compounded_reward.clone()
    }

    #[inline]
    fn get_initial_farming_tokens(&self) -> BigUint<M> {
        &self.current_farm_amount - &self.compounded_reward
    }
}
