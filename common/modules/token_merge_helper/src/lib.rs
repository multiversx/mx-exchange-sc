#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::ERROR_ZERO_AMOUNT;

#[derive(ManagedVecItem, Clone)]
pub struct ValueWeight<M: ManagedTypeApi> {
    pub value: BigUint<M>,
    pub weight: BigUint<M>,
}

pub enum WeightedAverageType {
    Floor,
    Ceil,
}

#[multiversx_sc::module]
pub trait TokenMergeHelperModule {
    fn weighted_average(
        &self,
        dataset: ManagedVec<ValueWeight<Self::Api>>,
        average_type: WeightedAverageType,
    ) -> BigUint {
        let mut weight_sum = BigUint::zero();
        let mut elem_weight_sum = BigUint::zero();
        for item in &dataset {
            weight_sum += &item.weight;
            elem_weight_sum += item.value * item.weight;
        }

        match average_type {
            WeightedAverageType::Floor => elem_weight_sum / weight_sum,
            WeightedAverageType::Ceil => (elem_weight_sum + &weight_sum - 1u64) / weight_sum,
        }
    }

    /// part * value / total
    fn rule_of_three(&self, part: &BigUint, total: &BigUint, value: &BigUint) -> BigUint {
        &(part * value) / total
    }

    /// part * value / total
    fn rule_of_three_non_zero_result(
        &self,
        part: &BigUint,
        total: &BigUint,
        value: &BigUint,
    ) -> BigUint {
        let res = &(part * value) / total;
        require!(res != 0u64, ERROR_ZERO_AMOUNT);
        res
    }
}
