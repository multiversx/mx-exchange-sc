use pausable::State;

use crate::errors::{ERROR_NOT_ACTIVE, ERROR_NOT_INITIALIZED};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const TICK_SPACING_COEF: u64 = 2;
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000;

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, PartialEq, Eq, TypeAbi, Clone)]
pub struct PoolState<M: ManagedTypeApi> {
    pub current_tick: i32,
    pub current_tick_node_id: u32,
    pub tick_spacing: u64,
    pub price_increase_per_tick: BigUint<M>,
    pub sqrt_price: BigUint<M>,
    pub virtual_liquidity: BigUint<M>,
    pub global_first_token_accumulated_fee: BigUint<M>,
    pub global_second_token_accumulated_fee: BigUint<M>,
    pub swap_fee_percentage: u64,
}

impl<M: ManagedTypeApi> Default for PoolState<M> {
    fn default() -> Self {
        Self {
            current_tick: 0i32,
            current_tick_node_id: 0u32,
            tick_spacing: 0u64,
            price_increase_per_tick: BigUint::zero(),
            sqrt_price: BigUint::zero(),
            virtual_liquidity: BigUint::zero(),
            global_first_token_accumulated_fee: BigUint::zero(),
            global_second_token_accumulated_fee: BigUint::zero(),
            swap_fee_percentage: 0u64,
        }
    }
}

impl<M: ManagedTypeApi> PoolState<M> {
    pub fn new(
        current_tick: i32,
        current_tick_node_id: u32,
        sqrt_price: BigUint<M>,
        price_increase_per_tick: BigUint<M>,
        swap_fee_percentage: u64,
    ) -> Self {
        Self {
            current_tick,
            current_tick_node_id,
            tick_spacing: swap_fee_percentage * TICK_SPACING_COEF,
            price_increase_per_tick,
            sqrt_price,
            virtual_liquidity: BigUint::zero(),
            global_first_token_accumulated_fee: BigUint::zero(),
            global_second_token_accumulated_fee: BigUint::zero(),
            swap_fee_percentage,
        }
    }
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, PartialEq, Eq, TypeAbi, Clone)]
pub struct Tick<M: ManagedTypeApi> {
    pub delta_liquidity_cross_up: BigInt<M>,
    pub first_token_accumulated_fee: BigUint<M>,
    pub second_token_accumulated_fee: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for Tick<M> {
    fn default() -> Self {
        Self {
            delta_liquidity_cross_up: BigInt::zero(),
            first_token_accumulated_fee: BigUint::zero(),
            second_token_accumulated_fee: BigUint::zero(),
        }
    }
}

#[multiversx_sc::module]
pub trait ConfigModule: permissions_module::PermissionsModule + pausable::PausableModule {
    fn is_pool_active(&self) {
        require!(self.is_state_active(self.state().get()), ERROR_NOT_ACTIVE);
        require!(!self.pool_state().is_empty(), ERROR_NOT_INITIALIZED);
    }

    #[inline]
    fn is_state_active(&self, state: State) -> bool {
        state == State::Active || state == State::PartialActive
    }

    #[inline]
    fn can_swap(&self, state: State) -> bool {
        state == State::Active
    }

    #[view(getPoolState)]
    #[storage_mapper("poolState")]
    fn pool_state(&self) -> SingleValueMapper<PoolState<Self::Api>>;

    #[view(getTicks)]
    #[storage_mapper("ticks")]
    fn ticks(&self, tick: i32) -> SingleValueMapper<Tick<Self::Api>>;

    #[storage_mapper("initializedTicks")]
    fn initialized_ticks(&self) -> LinkedListMapper<i32>;

    #[storage_mapper("ticksWhitelist")]
    fn ticks_whitelist(&self) -> WhitelistMapper<i32>;

    #[view(getFirstTokenId)]
    #[storage_mapper("firstTokenId")]
    fn first_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getSecondTokenId)]
    #[storage_mapper("secondTokenId")]
    fn second_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
