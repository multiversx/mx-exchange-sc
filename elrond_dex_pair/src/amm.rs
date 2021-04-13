elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module(AmmModuleImpl)]
pub trait AmmModule {
    fn calculate_k_constant(
        &self,
        first_token_amount: BigUint,
        second_token_amount: BigUint,
    ) -> BigUint {
        first_token_amount * second_token_amount
    }

    fn quote(
        &self,
        first_token_amount: BigUint,
        first_token_reserve: BigUint,
        second_token_reserve: BigUint,
    ) -> BigUint {
        (first_token_amount * second_token_reserve) / first_token_reserve
    }

    fn get_amount_out_no_fee(
        &self,
        amount_in: BigUint,
        reserve_in: BigUint,
        reserve_out: BigUint,
    ) -> BigUint {
        let numerator = amount_in.clone() * reserve_out;
        let denominator = reserve_in + amount_in;

        numerator / denominator
    }

    fn get_amount_out(
        &self,
        amount_in: BigUint,
        reserve_in: BigUint,
        reserve_out: BigUint,
    ) -> BigUint {
        let amount_in_with_fee = amount_in * BigUint::from(1000 - self.total_fee_precent().get());
        let numerator = amount_in_with_fee.clone() * reserve_out;
        let denominator = (reserve_in * BigUint::from(1000u64)) + amount_in_with_fee;

        numerator / denominator
    }

    fn get_amount_in(
        &self,
        amount_out: BigUint,
        reserve_in: BigUint,
        reserve_out: BigUint,
    ) -> BigUint {
        let numerator = (reserve_in * amount_out.clone()) * BigUint::from(1000u64);
        let denominator =
            (reserve_out - amount_out) * BigUint::from(1000 - self.total_fee_precent().get());

        (numerator / denominator) + BigUint::from(1u64)
    }

    fn get_special_fee_from_fixed_input(&self, amount_in: BigUint) -> BigUint {
        amount_in * BigUint::from(self.special_fee_precent().get()) / BigUint::from(1000u64)
    }

    fn get_special_fee_from_optimal_input(&self, amount_in_optimal: BigUint) -> BigUint {
        let amount_in_zero_fee = amount_in_optimal
            * BigUint::from(1000 - self.total_fee_precent().get())
            / BigUint::from(1000u64);

        amount_in_zero_fee.clone() * BigUint::from(1000u64)
            / BigUint::from(1000 - self.special_fee_precent().get())
            - amount_in_zero_fee
    }

    #[view(getTotalFeePrecent)]
    #[storage_mapper("total_fee_precent")]
    fn total_fee_precent(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getSpecialFeePrecent)]
    #[storage_mapper("special_fee_precent")]
    fn special_fee_precent(&self) -> SingleValueMapper<Self::Storage, u64>;
}
