#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(ManagedVecItem, Clone)]
pub struct ValueWeight<M: ManagedTypeApi> {
    pub value: BigUint<M>,
    pub weight: BigUint<M>,
}

#[elrond_wasm::module]
pub trait TokenMergeModule {
    fn rule_of_three(&self, part: &BigUint, total: &BigUint, value: &BigUint) -> BigUint {
        &(part * value) / total
    }

    fn rule_of_three_non_zero_result(
        &self,
        part: &BigUint,
        total: &BigUint,
        value: &BigUint,
    ) -> SCResult<BigUint> {
        let res = &(part * value) / total;
        require!(res != 0, "Rule of three result is zero");
        Ok(res)
    }

    fn weighted_average_ceil(&self, dataset: ManagedVec<ValueWeight<Self::Api>>) -> BigUint {
        let mut weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum = &elem_weight_sum + &(&x.value * &x.weight));

        (&elem_weight_sum + &weight_sum - 1u64) / weight_sum
    }
}
