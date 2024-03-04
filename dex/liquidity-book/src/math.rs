multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{PRICE_DECIMALS, PRICE_INCREASE_PER_TICK, PRICE_SCALING_FACTOR};

use super::config;

#[multiversx_sc::module]
pub trait MathModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn compute_liquidity(
        &self,
        first_token_amount: &BigUint,
        second_token_amount: &BigUint,
        current_price: &BigUint,
        min_price: &BigUint,
        max_price: &BigUint,
        scaling_factor: &BigUint,
    ) -> BigUint {
        let first_token_worth = first_token_amount;
        let second_token_worth = second_token_amount;
        let liquidity_by_first_token = self.liquidity_by_first_token(
            first_token_worth,
            scaling_factor,
            current_price,
            max_price,
        );

        let liquidity_by_second_token = self.liquidity_by_second_token(
            second_token_worth,
            scaling_factor,
            current_price,
            min_price,
        );

        if liquidity_by_first_token < liquidity_by_second_token {
            liquidity_by_first_token
        } else {
            liquidity_by_second_token
        }
    }

    fn compute_first_token_amount(
        &self,
        virtual_liquidity: &BigUint,
        current_price: &BigUint,
        other_price: &BigUint,
        scaling_factor: &BigUint,
    ) -> BigUint {
        if other_price > current_price {
            virtual_liquidity * scaling_factor * (other_price - current_price)
                / current_price
                / other_price
        } else {
            virtual_liquidity * scaling_factor * (current_price - other_price)
                / other_price
                / current_price
        }
    }

    fn compute_second_token_amount(
        &self,
        virtual_liquidity: &BigUint,
        current_price: &BigUint,
        other_price: &BigUint,
        scaling_factor: &BigUint,
    ) -> BigUint {
        let price = if other_price > current_price {
            virtual_liquidity * &(other_price - current_price) / scaling_factor
        } else {
            virtual_liquidity * &(current_price - other_price) / scaling_factor
        };

        price + BigUint::from(1u64)
    }

    fn liquidity_by_first_token(
        &self,
        token_amount: &BigUint,
        scaling_factor: &BigUint,
        current_price: &BigUint,
        other_price: &BigUint,
    ) -> BigUint {
        if current_price > other_price {
            (token_amount * &(current_price * other_price) / scaling_factor)
                / (current_price - other_price)
        } else {
            (token_amount * &(other_price * current_price) / scaling_factor)
                / (other_price - current_price)
        }
    }

    fn liquidity_by_second_token(
        &self,
        token_amount: &BigUint,
        scaling_factor: &BigUint,
        current_price: &BigUint,
        other_price: &BigUint,
    ) -> BigUint {
        if current_price > other_price {
            token_amount * scaling_factor / (current_price - other_price)
        } else {
            token_amount * scaling_factor / (other_price - current_price)
        }
    }

    fn tick_to_sqrtp(&self, tick: i32) -> BigUint {
        let price = self.tick_to_price(tick);
        self.price_to_sqrtp(&price)
    }

    // TODO
    // Update division_safety_constant logic -> it doesn't support numbers < 10^18
    fn tick_to_price(&self, tick: i32) -> BigUint {
        let division_safety_constant = BigUint::from(PRICE_DECIMALS);
        let price_base = BigFloat::from(PRICE_INCREASE_PER_TICK) / PRICE_SCALING_FACTOR.into();
        let price = price_base.pow(tick);
        let price_scaled_down = price.ceil().into_big_uint().unwrap_or_else(BigUint::zero);
        price_scaled_down * division_safety_constant
    }

    fn sqrtp_to_tick(&self, sqrt_price: &BigUint) -> i32 {
        let price = self.sqrtp_to_price(sqrt_price);
        let log_numerator = BigFloat::from(price);
        let log_base =
            BigFloat::from(PRICE_INCREASE_PER_TICK) / BigFloat::from(PRICE_SCALING_FACTOR);

        self.log_base_n(log_numerator, log_base)
    }

    // TODO
    // Update division_safety_constant logic -> it doesn't support numbers < 10^18
    // Here is needed to convert BigUint to BigFloat
    fn price_to_tick(&self, price: &BigUint) -> i32 {
        let division_safety_constant = BigUint::from(PRICE_DECIMALS);
        let price_scaled_down = price / &division_safety_constant;
        let log_numerator = BigFloat::from(price_scaled_down);
        let log_base =
            BigFloat::from(PRICE_INCREASE_PER_TICK) / BigFloat::from(PRICE_SCALING_FACTOR);

        self.log_base_n(log_numerator, log_base)
    }

    fn price_to_sqrtp(&self, price: &BigUint) -> BigUint {
        let division_safety_constant = BigUint::from(PRICE_DECIMALS);
        let scaling_factor = self.get_scaling_factor();
        (price * &scaling_factor).sqrt() * scaling_factor.sqrt() / division_safety_constant.sqrt()
    }

    fn sqrtp_to_price(&self, sqrt_price: &BigUint) -> BigUint {
        let division_safety_constant = BigUint::from(PRICE_DECIMALS);
        let scaling_factor = self.get_scaling_factor();
        sqrt_price.pow(2) / scaling_factor.pow(2) * division_safety_constant + BigUint::from(1u64)
    }

    fn log_base_n(&self, numerator: BigFloat, base: BigFloat) -> i32 {
        let mut result = 0;
        let mut num = numerator;
        while num >= base {
            num /= base.clone();
            result += 1;
        }
        result
    }

    fn add_liquidity_delta(
        &self,
        initial_liquidity: &BigUint,
        liquidity_delta: &BigUint,
    ) -> BigUint {
        initial_liquidity + liquidity_delta
    }

    fn remove_liquidity_delta(
        &self,
        initial_liquidity: &BigUint,
        liquidity_delta: &BigUint,
    ) -> BigUint {
        require!(
            initial_liquidity >= liquidity_delta,
            "Liquidity delta overflow"
        );
        initial_liquidity - liquidity_delta
    }

    // Q96 -> 2^96 (7922816251426433759)
    fn get_scaling_factor(&self) -> BigUint {
        BigUint::from(2u64).pow(96)
    }
}
