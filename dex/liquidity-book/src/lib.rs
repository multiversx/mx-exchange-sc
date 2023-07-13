#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
pub mod errors;
pub mod fees;
pub mod lp_token;
pub mod math;
pub mod tick;

use crate::{
    config::{PoolState, Tick},
    errors::*,
    lp_token::LpTokenAttributes,
};
use pausable::State;

pub type AddLiquidityResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub const MAX_PERCENTAGE: u64 = 10_000;
pub const PRICE_DECIMALS: u64 = 1_000_000_000_000_000_000;
pub const PRICE_INCREASE_PER_TICK: i32 = 10_001;
pub const PRICE_SCALING_FACTOR: i32 = 10_000;
pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = -MIN_TICK;

// Tick spacing options
// fee_amount 5 (0.05%) - tick_spacing = 10
// fee_amount 30 (0.3%) - tick_spacing = 60
// fee_amount 100 (1%) - tick_spacing = 200

#[multiversx_sc::contract]
pub trait LiquidityBook:
    config::ConfigModule
    + math::MathModule
    + tick::TickModule
    + fees::FeesModule
    + lp_token::LpTokenModule
    + token_send::TokenSendModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        deploy_price: BigUint,
        price_increase_per_tick: BigUint,
        swap_fee_percentage: u64,
    ) {
        require!(first_token_id.is_valid_esdt_identifier(), ERROR_NOT_AN_ESDT);
        require!(
            second_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(first_token_id != second_token_id, ERROR_SAME_TOKENS);
        self.first_token_id().set_if_empty(&first_token_id);
        self.second_token_id().set_if_empty(&second_token_id);
        self.state().set(State::Inactive);

        if self.pool_state().is_empty() {
            let sqrt_price = self.price_to_sqrtp(&deploy_price);
            let current_tick = self.price_to_tick(&deploy_price);
            let current_tick_node_id = self
                .initialized_ticks()
                .push_front(current_tick)
                .get_node_id();
            self.ticks_whitelist().add(&current_tick);
            self.ticks(current_tick).set_if_empty(Tick::default());
            let pool_state = PoolState::new(
                current_tick,
                current_tick_node_id,
                sqrt_price,
                price_increase_per_tick,
                swap_fee_percentage,
            );
            self.pool_state().set(pool_state);
        }
    }

    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        min_price: BigUint,
        max_price: BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        self.is_pool_active();
        let mut pool_state = self.pool_state().get();

        let tick_min = self.price_to_tick(&min_price);
        let tick_max = self.price_to_tick(&max_price);

        self.check_ticks(tick_min, tick_max, pool_state.tick_spacing);

        // TODO - add custom logic to populate initialized_ticks
        if !self.ticks_whitelist().contains(&tick_min) {
            self.ticks_whitelist().add(&tick_min);
            self.initialized_ticks().push_front(tick_min);
        }
        if !self.ticks_whitelist().contains(&tick_max) {
            self.ticks_whitelist().add(&tick_max);
            self.initialized_ticks().push_back(tick_max);
        }

        let caller = self.blockchain().get_caller();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let [mut first_payment, mut second_payment] = self.call_value().multi_esdt();
        require!(
            first_payment.token_identifier == first_token_id
                && second_payment.token_identifier == second_token_id
                && first_payment.amount > 0
                && second_payment.amount > 0,
            ERROR_BAD_PAYMENT_TOKENS
        );

        let scaling_factor = self.get_scaling_factor();
        let current_price = pool_state.sqrt_price.clone();
        let min_price = self.tick_to_sqrtp(tick_min);
        let max_price = self.tick_to_sqrtp(tick_max);

        let virtual_liquidity = self.compute_liquidity(
            &first_payment.amount,
            &second_payment.amount,
            &current_price,
            &min_price,
            &max_price,
            &scaling_factor,
        );

        // Price in range <=> tick_min <= tick_current < tick_max
        let target_price = if current_price < min_price {
            &min_price
        } else if current_price >= max_price {
            &max_price
        } else {
            &current_price
        };

        let first_token_actual_amount = self.compute_first_token_amount(
            &virtual_liquidity,
            &current_price,
            &max_price,
            &scaling_factor,
        );

        let second_token_actual_amount = self.compute_second_token_amount(
            &virtual_liquidity,
            &current_price,
            &min_price,
            &scaling_factor,
        );

        require!(
            first_payment.amount >= first_token_actual_amount,
            ERROR_INSUFFICIENT_FIRST_TOKEN
        );

        require!(
            second_payment.amount >= second_token_actual_amount,
            ERROR_INSUFFICIENT_SECOND_TOKEN
        );

        let tick_min_mapper = self.get_tick_mapper(tick_min);
        let tick_max_mapper = self.get_tick_mapper(tick_max);

        tick_min_mapper.update(|tick| {
            tick.delta_liquidity_cross_up += BigInt::from(virtual_liquidity.clone());
        });
        tick_max_mapper.update(|tick| {
            tick.delta_liquidity_cross_up += BigInt::from(virtual_liquidity.clone());
        });
        if target_price == &current_price {
            pool_state.virtual_liquidity += virtual_liquidity.clone();
        }

        let (first_token_total_fees, second_token_total_fees) = self.compute_fees(
            &current_price,
            &min_price,
            &max_price,
            tick_min_mapper.get(),
            tick_max_mapper.get(),
            &pool_state,
        );

        let new_attributes = LpTokenAttributes {
            virtual_liquidity: virtual_liquidity.clone(),
            tick_min,
            tick_max,
            first_token_accumulated_fee: first_token_total_fees,
            second_token_accumulated_fee: second_token_total_fees,
        };
        let lp_token_payment = self.mint_lp_tokens(virtual_liquidity, &new_attributes);
        self.pool_state().set(pool_state);

        first_payment.amount -= first_token_actual_amount; //first_token_remaining_dust
        second_payment.amount -= second_token_actual_amount; //first_token_remaining_dust
        let mut output_payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();
        output_payments.push(lp_token_payment.clone());
        output_payments.push(first_payment.clone());
        output_payments.push(second_payment.clone());

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        (lp_token_payment, first_payment, second_payment).into()
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(&self) -> RemoveLiquidityResultType<Self::Api> {
        self.is_pool_active();
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        require!(
            payment.token_identifier == self.lp_token().get_token_id(),
            ERROR_BAD_PAYMENT_TOKENS
        );

        let token_attributes: LpTokenAttributes<Self::Api> =
            self.get_lp_token_attributes(payment.token_nonce);

        let scaling_factor = self.get_scaling_factor();
        let mut pool_state = self.pool_state().get();
        let current_price = pool_state.sqrt_price.clone();
        let min_price = self.tick_to_sqrtp(token_attributes.tick_min);
        let max_price = self.tick_to_sqrtp(token_attributes.tick_max);

        // Price in range <=> tick_min <= tick_current < tick_max
        let target_price = if current_price < min_price {
            &min_price
        } else if current_price >= max_price {
            &max_price
        } else {
            &current_price
        };

        let first_token_actual_amount = self.compute_first_token_amount(
            &payment.amount,
            &current_price,
            &max_price,
            &scaling_factor,
        );

        let second_token_actual_amount = self.compute_second_token_amount(
            &payment.amount,
            &current_price,
            &min_price,
            &scaling_factor,
        );

        let tick_min_mapper = self.get_tick_mapper(token_attributes.tick_min);
        let tick_max_mapper = self.get_tick_mapper(token_attributes.tick_max);
        tick_min_mapper.update(|tick| {
            tick.delta_liquidity_cross_up -= BigInt::from(payment.amount.clone());
            if tick.delta_liquidity_cross_up == 0 {}
        });
        tick_max_mapper.update(|tick| {
            tick.delta_liquidity_cross_up += BigInt::from(payment.amount.clone());
            if tick.delta_liquidity_cross_up == 0 {}
        });
        if target_price == &current_price {
            pool_state.virtual_liquidity -= payment.amount.clone();
        }

        // Compute fees
        let (first_token_total_fees, second_token_total_fees) = self.compute_fees(
            &current_price,
            &min_price,
            &max_price,
            tick_min_mapper.get(),
            tick_max_mapper.get(),
            &pool_state,
        );
        let first_token_user_fees = &payment.amount
            * &(first_token_total_fees - token_attributes.first_token_accumulated_fee);
        let second_token_user_fees = &payment.amount
            * &(second_token_total_fees - token_attributes.second_token_accumulated_fee);

        let first_token_actual_user_fees =
            &first_token_user_fees * &pool_state.virtual_liquidity / &scaling_factor;

        let second_token_actual_user_fees =
            &second_token_user_fees * &pool_state.virtual_liquidity / &scaling_factor;

        self.burn_lp_tokens(payment.token_nonce, &payment.amount);
        self.uninitialize_tick_if_needed(tick_min_mapper);
        self.uninitialize_tick_if_needed(tick_max_mapper);
        self.pool_state().set(pool_state);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let mut output_payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();
        let first_token_payment =
            EsdtTokenPayment::new(first_token_id.clone(), 0, first_token_actual_amount);
        let second_token_payment =
            EsdtTokenPayment::new(second_token_id.clone(), 0, second_token_actual_amount);
        let first_token_fee_payment =
            EsdtTokenPayment::new(first_token_id, 0, first_token_actual_user_fees);
        let second_token_fee_payment =
            EsdtTokenPayment::new(second_token_id, 0, second_token_actual_user_fees);

        output_payments.push(first_token_payment.clone());
        output_payments.push(second_token_payment.clone());
        output_payments.push(first_token_fee_payment);
        output_payments.push(second_token_fee_payment);

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);
        (first_token_payment, second_token_payment).into()
    }

    #[payable("*")]
    #[endpoint(swapTokens)]
    fn swap_tokens(&self) -> EsdtTokenPayment {
        self.is_pool_active();
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let output_token;
        let first_for_second;
        if payment.token_identifier == first_token_id {
            first_for_second = true;
            output_token = second_token_id;
        } else if payment.token_identifier == second_token_id {
            first_for_second = false;
            output_token = first_token_id;
        } else {
            sc_panic!(ERROR_BAD_PAYMENT_TOKENS)
        }

        let scaling_factor = self.get_scaling_factor();
        let pool_state_mapper = self.pool_state();
        let mut pool_state = pool_state_mapper.get();

        require!(pool_state.virtual_liquidity > 0, ERROR_NOT_ENOUGH_LP);
        let mut amount_left_to_swap = payment.amount;
        let mut user_output_amount = BigUint::zero();

        // Swap first token
        let mut current_tick_node;
        while amount_left_to_swap > 0 {
            current_tick_node = self
                .initialized_ticks()
                .get_node_by_id(pool_state.current_tick_node_id)
                .unwrap();
            let next_tick_node_id = if first_for_second {
                current_tick_node.get_prev_node_id()
            } else {
                current_tick_node.get_next_node_id()
            };
            let next_tick = self
                .initialized_ticks()
                .get_node_by_id(next_tick_node_id)
                .unwrap()
                .get_value_cloned();
            let next_sqrt_price = self.tick_to_sqrtp(next_tick);
            let current_price = &pool_state.sqrt_price;
            let amount_left_to_swap_without_fees = amount_left_to_swap.clone()
                * (MAX_PERCENTAGE - pool_state.swap_fee_percentage)
                / MAX_PERCENTAGE;
            let target_price = if first_for_second {
                // (virtual_liquidity * q96 * current_sqrt_price) / (virtual_liquidity * q96 + amount_in * current_sqrt_price)
                let computed_price =
                    (&pool_state.virtual_liquidity * &scaling_factor * current_price)
                        / (&pool_state.virtual_liquidity * &scaling_factor
                            + &amount_left_to_swap_without_fees * current_price);

                if computed_price < next_sqrt_price {
                    next_sqrt_price.clone()
                } else {
                    computed_price
                }
            } else {
                // current_price + (amount_in * q96) / virtual_liquidity
                let computed_price = current_price
                    + &(&amount_left_to_swap_without_fees * &scaling_factor
                        / &pool_state.virtual_liquidity);

                if computed_price > next_sqrt_price {
                    next_sqrt_price.clone()
                } else {
                    computed_price
                }
            };

            let first_token_amount = self.compute_first_token_amount(
                &pool_state.virtual_liquidity,
                current_price,
                &target_price,
                &scaling_factor,
            );

            let second_token_amount = self.compute_second_token_amount(
                &pool_state.virtual_liquidity,
                current_price,
                &target_price,
                &scaling_factor,
            );

            let (input_token, output_token) = if first_for_second {
                (first_token_amount, second_token_amount)
            } else {
                (second_token_amount, first_token_amount)
            };

            // Compute fee
            let fee_amount = if next_sqrt_price == target_price {
                let mut computed_fee_amount = &input_token * pool_state.swap_fee_percentage
                    / (MAX_PERCENTAGE - pool_state.swap_fee_percentage);

                // Round up only if it doesn't overflow
                if &computed_fee_amount + &input_token < amount_left_to_swap {
                    computed_fee_amount += BigUint::from(1u64);
                }

                computed_fee_amount
            } else {
                &amount_left_to_swap - &input_token
            };

            let computed_global_fee = &fee_amount * &scaling_factor / &pool_state.virtual_liquidity;
            if first_for_second {
                pool_state.global_first_token_accumulated_fee += computed_global_fee;
            } else {
                pool_state.global_second_token_accumulated_fee += computed_global_fee;
            }

            // Compute amounts
            user_output_amount += output_token;
            amount_left_to_swap -= input_token;
            amount_left_to_swap -= fee_amount;

            pool_state.sqrt_price = target_price;
            if amount_left_to_swap > 0 {
                if first_for_second {
                    self.cross_tick_downwards(&mut pool_state, next_tick, next_tick_node_id);
                } else {
                    self.cross_tick_upwards(&mut pool_state, next_tick, next_tick_node_id);
                }
            }
        }

        pool_state_mapper.set(pool_state);
        self.send_tokens_non_zero(&caller, &output_token, 0, &user_output_amount);

        EsdtTokenPayment::new(output_token, 0, user_output_amount)
    }
}
