elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::config;

#[elrond_wasm::module]
pub trait AmmModule: config::ConfigModule + token_send::TokenSendModule {
    fn calculate_k_constant(
        &self,
        first_token_amount: &Self::BigUint,
        second_token_amount: &Self::BigUint,
    ) -> Self::BigUint {
        first_token_amount * second_token_amount
    }

    fn quote(
        &self,
        first_token_amount: &Self::BigUint,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) -> Self::BigUint {
        &(first_token_amount * second_token_reserve) / first_token_reserve
    }

    fn get_amount_out_no_fee(
        &self,
        amount_in: &Self::BigUint,
        reserve_in: &Self::BigUint,
        reserve_out: &Self::BigUint,
    ) -> Self::BigUint {
        let numerator = amount_in * reserve_out;
        let denominator = reserve_in + amount_in;

        numerator / denominator
    }

    fn get_amount_out(
        &self,
        amount_in: &Self::BigUint,
        reserve_in: &Self::BigUint,
        reserve_out: &Self::BigUint,
    ) -> Self::BigUint {
        let amount_in_with_fee = amount_in * &(100000 - self.total_fee_percent().get()).into();
        let numerator = &amount_in_with_fee * reserve_out;
        let denominator = (reserve_in * &100000u64.into()) + amount_in_with_fee;

        numerator / denominator
    }

    fn get_amount_in(
        &self,
        amount_out: &Self::BigUint,
        reserve_in: &Self::BigUint,
        reserve_out: &Self::BigUint,
    ) -> Self::BigUint {
        let numerator = reserve_in * amount_out * 100000u64.into();
        let denominator =
            (reserve_out - amount_out) * (100000 - self.total_fee_percent().get()).into();

        (numerator / denominator) + 1u64.into()
    }

    fn get_special_fee_from_input(&self, amount_in: &Self::BigUint) -> Self::BigUint {
        amount_in * &self.special_fee_percent().get().into() / 100000u64.into()
    }
}
