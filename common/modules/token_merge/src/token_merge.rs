#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::ERROR_ZERO_AMOUNT;

#[derive(ManagedVecItem, Clone)]
pub struct ValueWeight<M: ManagedTypeApi> {
    pub value: BigUint<M>,
    pub weight: BigUint<M>,
}

#[elrond_wasm::module]
pub trait TokenMergeModule {
    fn weighted_average(&self, dataset: ManagedVec<ValueWeight<Self::Api>>) -> BigUint {
        let mut weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum += &x.value * &x.weight);

        elem_weight_sum / weight_sum
    }

    fn weighted_average_ceil(&self, dataset: ManagedVec<ValueWeight<Self::Api>>) -> BigUint {
        let mut weight_sum = BigUint::zero();
        dataset.iter().for_each(|x| weight_sum += &x.weight);

        let mut elem_weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum += &x.value * &x.weight);

        (&elem_weight_sum + &weight_sum - 1u64) / weight_sum
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
