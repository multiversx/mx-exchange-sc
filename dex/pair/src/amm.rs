elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::config;

#[elrond_wasm::module]
pub trait AmmModule: config::ConfigModule + token_send::TokenSendModule {
    fn calculate_k_constant(
        &self,
        first_token_amount: &BigUint,
        second_token_amount: &BigUint,
    ) -> BigUint {
        first_token_amount * second_token_amount
    }

    fn quote(
        &self,
        first_token_amount: &BigUint,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
    ) -> BigUint {
        &(first_token_amount * second_token_reserve) / first_token_reserve
    }

    fn get_amount_out_no_fee(
        &self,
        amount_in: &BigUint,
        reserve_in: &BigUint,
        reserve_out: &BigUint,
    ) -> BigUint {
        let numerator = amount_in * reserve_out;
        let denominator = reserve_in + amount_in;

        numerator / denominator
    }

    fn get_amount_out(
        &self,
        amount_in: &BigUint,
        reserve_in: &BigUint,
        reserve_out: &BigUint,
    ) -> BigUint {
        let amount_in_with_fee = amount_in * (100000 - self.total_fee_percent().get());
        let numerator = &amount_in_with_fee * reserve_out;
        let denominator = (reserve_in * 100000u64) + amount_in_with_fee;

        numerator / denominator
    }

    fn get_amount_in(
        &self,
        amount_out: &BigUint,
        reserve_in: &BigUint,
        reserve_out: &BigUint,
    ) -> BigUint {
        let numerator = reserve_in * amount_out * 100000u64;
        let denominator = (reserve_out - amount_out) * (100000 - self.total_fee_percent().get());

        (numerator / denominator) + 1u64
    }

    fn get_special_fee_from_input(&self, amount_in: &BigUint) -> BigUint {
        amount_in * self.special_fee_percent().get() / 100000u64
    }
}
