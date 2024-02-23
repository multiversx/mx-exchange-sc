multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{Epoch, FarmToken, FarmTokenAttributes, Nonce};
use fixed_supply_token::FixedSupplyToken;
use math::weighted_average_round_up;
use mergeable::Mergeable;

static NOT_IMPLEMENTED_ERR_MSG: &[u8] = b"Not implemented";

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
pub struct FarmTokenConcentratedLiqAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub entering_epoch: Epoch,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
    pub original_owner: ManagedAddress<M>,
    pub lp_token_nonce: Nonce,
}

impl<M: ManagedTypeApi> From<FarmTokenAttributes<M>> for FarmTokenConcentratedLiqAttributes<M> {
    fn from(_value: FarmTokenAttributes<M>) -> Self {
        M::error_api_impl().signal_error(NOT_IMPLEMENTED_ERR_MSG);
    }
}

impl<M: ManagedTypeApi> Into<FarmTokenAttributes<M>> for FarmTokenConcentratedLiqAttributes<M> {
    fn into(self) -> FarmTokenAttributes<M> {
        M::error_api_impl().signal_error(NOT_IMPLEMENTED_ERR_MSG);
    }
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for FarmTokenConcentratedLiqAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.current_farm_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_compounded_reward = self.rule_of_three(payment_amount, &self.compounded_reward);
        let new_current_farm_amount = payment_amount.clone();

        FarmTokenConcentratedLiqAttributes {
            reward_per_share: self.reward_per_share,
            entering_epoch: self.entering_epoch,
            compounded_reward: new_compounded_reward,
            current_farm_amount: new_current_farm_amount,
            original_owner: self.original_owner,
            lp_token_nonce: self.lp_token_nonce,
        }
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for FarmTokenConcentratedLiqAttributes<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        self.original_owner == other.original_owner && self.lp_token_nonce == other.lp_token_nonce
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

impl<M: ManagedTypeApi> FarmToken<M> for FarmTokenConcentratedLiqAttributes<M> {
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
