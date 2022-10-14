elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use fixed_supply_token::FixedSupplyToken;
use mergeable::{weighted_average, Mergeable};

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

impl<M: ManagedTypeApi> FixedSupplyToken<M> for FarmTokenAttributes<M> {
    fn get_total_supply(&self) -> &BigUint<M> {
        &self.current_farm_amount
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == self.get_total_supply() {
            return self;
        }

        // let new_rps = self.rule_of_three_non_zero_result(payment_amount, &self.reward_per_share);
        let new_initial_farming_amount =
            self.rule_of_three_non_zero_result(payment_amount, &self.initial_farming_amount);
        let new_compounded_reward =
            self.rule_of_three_non_zero_result(payment_amount, &self.compounded_reward);
        let new_current_farm_amount = payment_amount.clone();

        FarmTokenAttributes {
            reward_per_share: self.reward_per_share,
            original_entering_epoch: self.original_entering_epoch,
            entering_epoch: self.entering_epoch,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compounded_reward,
            current_farm_amount: new_current_farm_amount,
        }
    }
}

impl<M: ManagedTypeApi + BlockchainApi> Mergeable<M> for FarmTokenAttributes<M> {
    /// farm tokens can always be merged with each other
    #[inline]
    fn can_merge_with(&self, _other: &Self) -> bool {
        true
    }

    fn merge_with(&mut self, other: Self) {
        let first_supply = self.get_total_supply();
        let second_supply = other.get_total_supply();
        self.reward_per_share = weighted_average(
            &self.reward_per_share,
            first_supply,
            &other.reward_per_share,
            second_supply,
        );

        self.initial_farming_amount += other.initial_farming_amount;
        self.compounded_reward += other.compounded_reward;
        self.current_farm_amount += other.current_farm_amount;

        let current_epoch = M::blockchain_api_impl().get_block_epoch();
        self.original_entering_epoch = current_epoch;
        self.entering_epoch = current_epoch;
    }
}

pub trait FarmToken<M: ManagedTypeApi> {
    fn get_reward_per_share(&self) -> &BigUint<M>;

    fn get_compounded_rewards(&self) -> &BigUint<M>;

    fn get_initial_farming_tokens(&self) -> &BigUint<M>;
}

impl<M: ManagedTypeApi> FarmToken<M> for FarmTokenAttributes<M> {
    #[inline]
    fn get_reward_per_share(&self) -> &BigUint<M> {
        &self.reward_per_share
    }

    #[inline]
    fn get_compounded_rewards(&self) -> &BigUint<M> {
        &self.compounded_reward
    }

    #[inline]
    fn get_initial_farming_tokens(&self) -> &BigUint<M> {
        &self.initial_farming_amount
    }
}
