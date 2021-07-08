#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(Clone, Copy)]
pub struct ValueWeight<BigUint: BigUintApi> {
    pub value: BigUint,
    pub weight: BigUint,
}

#[elrond_wasm_derive::module]
pub trait TokenMergeModule {
    fn rule_of_three(
        &self,
        part: &Self::BigUint,
        total: &Self::BigUint,
        value: &Self::BigUint,
    ) -> Self::BigUint {
        &(part * value) / total
    }

    fn weighted_average(&self, dataset: Vec<ValueWeight<Self::BigUint>>) -> Self::BigUint {
        let mut weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum = &elem_weight_sum + &(&x.value * &x.weight));

        elem_weight_sum / weight_sum
    }

    fn weighted_average_ceil(&self, dataset: Vec<ValueWeight<Self::BigUint>>) -> Self::BigUint {
        let mut weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum = &elem_weight_sum + &(&x.value * &x.weight));

        (&elem_weight_sum + &weight_sum - Self::BigUint::from(1u64)) / weight_sum
    }
}
