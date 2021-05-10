elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait AmmModule {
    fn calculate_k_constant(
        &self,
        first_token_amount: Self::BigUint,
        second_token_amount: Self::BigUint,
    ) -> Self::BigUint {
        first_token_amount * second_token_amount
    }

    fn quote(
        &self,
        first_token_amount: Self::BigUint,
        first_token_reserve: Self::BigUint,
        second_token_reserve: Self::BigUint,
    ) -> Self::BigUint {
        (first_token_amount * second_token_reserve) / first_token_reserve
    }

    fn get_amount_out_no_fee(
        &self,
        amount_in: Self::BigUint,
        reserve_in: Self::BigUint,
        reserve_out: Self::BigUint,
    ) -> Self::BigUint {
        let numerator = &amount_in * &reserve_out;
        let denominator = reserve_in + amount_in;

        numerator / denominator
    }

    fn get_amount_out(
        &self,
        amount_in: Self::BigUint,
        reserve_in: Self::BigUint,
        reserve_out: Self::BigUint,
    ) -> Self::BigUint {
        let amount_in_with_fee =
            amount_in * Self::BigUint::from(100000 - self.total_fee_precent().get());
        let numerator = &amount_in_with_fee * &reserve_out;
        let denominator = (reserve_in * Self::BigUint::from(100000u64)) + amount_in_with_fee;

        numerator / denominator
    }

    fn get_amount_in(
        &self,
        amount_out: Self::BigUint,
        reserve_in: Self::BigUint,
        reserve_out: Self::BigUint,
    ) -> Self::BigUint {
        let numerator = (&reserve_in * &amount_out) * Self::BigUint::from(100000u64);
        let denominator = (reserve_out - amount_out)
            * Self::BigUint::from(100000 - self.total_fee_precent().get());

        (numerator / denominator) + Self::BigUint::from(1u64)
    }

    fn get_special_fee_from_fixed_input(&self, amount_in: Self::BigUint) -> Self::BigUint {
        amount_in * Self::BigUint::from(self.special_fee_precent().get())
            / Self::BigUint::from(100000u64)
    }

    fn get_special_fee_from_optimal_input(
        &self,
        amount_in_optimal: Self::BigUint,
    ) -> Self::BigUint {
        let amount_in_zero_fee = amount_in_optimal
            * Self::BigUint::from(100000 - self.total_fee_precent().get())
            / Self::BigUint::from(100000u64);

        &amount_in_zero_fee * &Self::BigUint::from(100000u64)
            / Self::BigUint::from(100000 - self.special_fee_precent().get())
            - amount_in_zero_fee
    }

    #[view(getTotalFeePrecent)]
    #[storage_mapper("total_fee_precent")]
    fn total_fee_precent(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getSpecialFeePrecent)]
    #[storage_mapper("special_fee_precent")]
    fn special_fee_precent(&self) -> SingleValueMapper<Self::Storage, u64>;
}
