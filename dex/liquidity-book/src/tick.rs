multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    config::{PoolState, Tick},
    errors::ERROR_INVALID_TICK_RANGE,
    MAX_TICK, MIN_TICK,
};

use super::config;

#[multiversx_sc::module]
pub trait TickModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn cross_tick_upwards(
        &self,
        pool_state: &mut PoolState<Self::Api>,
        next_tick: i32,
        next_tick_node_id: u32,
    ) {
        pool_state.current_tick = next_tick;
        pool_state.current_tick_node_id = next_tick_node_id;

        let tick_liquidity = self
            .ticks(pool_state.current_tick)
            .get()
            .delta_liquidity_cross_up
            .magnitude();
        pool_state.virtual_liquidity += tick_liquidity;

        self.ticks(pool_state.current_tick).update(|tick| {
            tick.first_token_accumulated_fee =
                &pool_state.global_first_token_accumulated_fee - &tick.first_token_accumulated_fee;
            tick.second_token_accumulated_fee = &pool_state.global_second_token_accumulated_fee
                - &tick.second_token_accumulated_fee;
        });
    }

    fn cross_tick_downwards(
        &self,
        pool_state: &mut PoolState<Self::Api>,
        next_tick: i32,
        next_tick_node_id: u32,
    ) {
        let tick_liquidity = self
            .ticks(pool_state.current_tick)
            .get()
            .delta_liquidity_cross_up
            .magnitude();
        // TODO - check decrementation
        pool_state.virtual_liquidity -= tick_liquidity;

        self.ticks(pool_state.current_tick).update(|tick| {
            tick.first_token_accumulated_fee =
                &pool_state.global_first_token_accumulated_fee - &tick.first_token_accumulated_fee;
            tick.second_token_accumulated_fee = &pool_state.global_second_token_accumulated_fee
                - &tick.second_token_accumulated_fee;
        });

        pool_state.current_tick = next_tick;
        pool_state.current_tick_node_id = next_tick_node_id;
    }

    fn check_ticks(&self, tick_min: i32, tick_max: i32, _tick_spacing: u64) {
        require!(
            tick_max >= tick_min && tick_min >= MIN_TICK && tick_max <= MAX_TICK,
            ERROR_INVALID_TICK_RANGE
        );
        // TODO
        // require!(
        //     tick_min % tick_spacing == 0, ERROR_INVALID_TICK_RANGE
        // );
        // require!(
        //     tick_max % tick_spacing == 0, ERROR_INVALID_TICK_RANGE
        // );
    }

    fn get_tick_mapper(&self, tick: i32) -> SingleValueMapper<Tick<Self::Api>> {
        let tick_mapper = self.ticks(tick);
        tick_mapper.set_if_empty(Tick::default());
        tick_mapper
    }

    fn uninitialize_tick_if_needed(&self, tick_mapper: SingleValueMapper<Tick<Self::Api>>) {
        if tick_mapper.get().delta_liquidity_cross_up == 0 {
            tick_mapper.clear();
        }
    }
}
