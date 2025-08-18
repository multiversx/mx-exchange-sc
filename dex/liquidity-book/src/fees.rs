multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::config::{PoolState, Tick};

use super::config;

#[multiversx_sc::module]
pub trait FeesModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn compute_fees(
        &self,
        current_price: &BigUint,
        min_price: &BigUint,
        max_price: &BigUint,
        tick_min_data: Tick<Self::Api>,
        tick_max_data: Tick<Self::Api>,
        pool_state: &PoolState<Self::Api>,
    ) -> (BigUint, BigUint) {
        let first_token_total_fee_above_max_tick;
        let second_token_total_fee_above_max_tick;
        let first_token_total_fee_below_min_tick;
        let second_token_total_fee_below_min_tick;

        if current_price < max_price {
            first_token_total_fee_above_max_tick = tick_max_data.first_token_accumulated_fee;
            second_token_total_fee_above_max_tick = tick_max_data.second_token_accumulated_fee;
        } else {
            first_token_total_fee_above_max_tick = &pool_state.global_first_token_accumulated_fee
                - &tick_max_data.first_token_accumulated_fee;
            second_token_total_fee_above_max_tick = &pool_state.global_second_token_accumulated_fee
                - &tick_max_data.second_token_accumulated_fee;
        };

        if current_price > min_price {
            first_token_total_fee_below_min_tick = tick_min_data.first_token_accumulated_fee;
            second_token_total_fee_below_min_tick = tick_min_data.second_token_accumulated_fee;
        } else {
            first_token_total_fee_below_min_tick = &pool_state.global_first_token_accumulated_fee
                - &tick_min_data.first_token_accumulated_fee;
            second_token_total_fee_below_min_tick = &pool_state.global_second_token_accumulated_fee
                - &tick_min_data.second_token_accumulated_fee;
        }

        let first_token_total_fees = &pool_state.global_first_token_accumulated_fee
            - &first_token_total_fee_above_max_tick
            - &first_token_total_fee_below_min_tick;
        let second_token_total_fees = &pool_state.global_second_token_accumulated_fee
            - &second_token_total_fee_above_max_tick
            - &second_token_total_fee_below_min_tick;

        (first_token_total_fees, second_token_total_fees)
    }
}
