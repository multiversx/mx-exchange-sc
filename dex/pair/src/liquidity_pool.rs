elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use crate::assert;
use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::base::Context;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::errors::*;

use super::amm;
use super::config;

const MINIMUM_LIQUIDITY: u64 = 1_000;

#[elrond_wasm::module]
pub trait LiquidityPoolModule:
    amm::AmmModule + config::ConfigModule + token_send::TokenSendModule
{
    fn pool_add_liquidity(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let zero = &BigUint::zero();
        assert!(
            self,
            context.get_lp_token_supply() != zero,
            ERROR_ZERO_AMOUNT
        );

        let first_payment_amount = context.get_first_amount_optimal();
        let second_payment_amount = context.get_second_amount_optimal();
        let lp_token_supply = context.get_lp_token_supply();
        let first_token_reserve = context.get_first_token_reserve();
        let second_token_reserve = context.get_second_token_reserve();

        let liquidity = self.biguint_min(
            &(&(first_payment_amount * lp_token_supply) / first_token_reserve),
            &(&(second_payment_amount * lp_token_supply) / second_token_reserve),
        );
        assert!(self, &liquidity > zero, ERROR_INSUFFICIENT_LIQUIDITY);

        context.increase_lp_token_supply(&liquidity);
        context.set_liquidity_added(liquidity);
        context.increase_reserves();
    }

    fn pool_add_initial_liquidity(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let zero = &BigUint::zero();
        assert!(
            self,
            context.get_lp_token_supply() == zero,
            ERROR_ZERO_AMOUNT,
        );

        let liquidity = self.biguint_min(
            context.get_first_amount_optimal(),
            context.get_second_amount_optimal(),
        );
        let minimum_liquidity = BigUint::from(MINIMUM_LIQUIDITY);
        assert!(self, liquidity > minimum_liquidity, ERROR_FIRST_LIQUDITY);

        let lpt = context.get_lp_token_id();
        self.send().esdt_local_mint(lpt, 0, &minimum_liquidity);

        context.set_liquidity_added(&liquidity - &minimum_liquidity);
        context.set_lp_token_supply(liquidity);
        context.increase_reserves();
    }

    fn pool_remove_liquidity(&self, context: &mut RemoveLiquidityContext<Self::Api>) {
        let (first_amount_removed, second_amounts_removed) = self.get_amounts_removed(context);
        context.set_first_token_amount_removed(first_amount_removed);
        context.set_second_token_amount_removed(second_amounts_removed);
        context.decrease_lp_token_supply();
        context.decrease_reserves();
    }

    fn get_amounts_removed(
        &self,
        context: &mut RemoveLiquidityContext<Self::Api>,
    ) -> (BigUint, BigUint) {
        let total_supply = context.get_lp_token_supply();
        let liquidity = &context.get_lp_token_payment().amount;

        assert!(
            self,
            total_supply >= &(liquidity + MINIMUM_LIQUIDITY),
            ERROR_NOT_ENOUGH_LP
        );

        let first_amount_removed = (liquidity * context.get_first_token_reserve()) / total_supply;
        assert!(
            self,
            first_amount_removed > 0u64,
            ERROR_INSUFFICIENT_LIQ_BURNED
        );
        assert!(
            self,
            &first_amount_removed >= context.get_first_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        assert!(
            self,
            context.get_first_token_reserve() > &first_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let second_amount_removed = (liquidity * context.get_second_token_reserve()) / total_supply;
        assert!(
            self,
            second_amount_removed > 0u64,
            ERROR_INSUFFICIENT_LIQ_BURNED
        );
        assert!(
            self,
            &second_amount_removed >= context.get_second_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        assert!(
            self,
            context.get_second_token_reserve() > &second_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        (first_amount_removed, second_amount_removed)
    }

    fn calculate_optimal_amounts(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let (first_amount_optimal, second_amount_optimal) = self.get_optimal_amounts(context);
        context.set_first_amount_optimal(first_amount_optimal);
        context.set_second_amount_optimal(second_amount_optimal);
    }

    fn set_initial_liquidity_optimals(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let first_amount_optimal = context.get_first_payment().amount.clone();
        let second_amount_optimal = context.get_second_payment().amount.clone();
        context.set_first_amount_optimal(first_amount_optimal);
        context.set_second_amount_optimal(second_amount_optimal);
    }

    fn get_optimal_amounts(
        &self,
        context: &mut AddLiquidityContext<Self::Api>,
    ) -> (BigUint, BigUint) {
        let zero = &BigUint::zero();
        let first_token_reserve = context.get_first_token_reserve();
        let second_token_reserve = context.get_second_token_reserve();
        let first_token_amount_desired = &context.get_first_payment().amount;
        let second_token_amount_desired = &context.get_first_payment().amount;
        let first_token_amount_min = context.get_first_token_amount_min();
        let second_token_amount_min = context.get_second_token_amount_min();
        assert!(
            self,
            first_token_reserve != zero && second_token_reserve != zero,
            ERROR_INITIAL_LIQUIDITY_NOT_ADDED,
        );

        let second_token_amount_optimal = self.quote(
            first_token_amount_desired,
            first_token_reserve,
            second_token_reserve,
        );

        if &second_token_amount_optimal <= second_token_amount_desired {
            assert!(
                self,
                &second_token_amount_optimal >= second_token_amount_min,
                ERROR_INSUFFICIENT_SECOND_TOKEN,
            );

            (
                first_token_amount_desired.clone(),
                second_token_amount_optimal.clone(),
            )
        } else {
            let first_token_amount_optimal = self.quote(
                second_token_amount_desired,
                second_token_reserve,
                first_token_reserve,
            );
            assert!(
                self,
                &first_token_amount_optimal <= first_token_amount_desired,
                ERROR_OPTIMAL_GRATER_THAN_PAID,
            );
            assert!(
                self,
                &first_token_amount_optimal >= first_token_amount_min,
                ERROR_INSUFFICIENT_FIRST_TOKEN,
            );

            (
                first_token_amount_optimal.clone(),
                second_token_amount_desired.clone(),
            )
        }
    }

    fn get_token_for_given_position(
        &self,
        liquidity: BigUint,
        token_id: TokenIdentifier,
    ) -> EsdtTokenPayment<Self::Api> {
        let reserve = self.pair_reserve(&token_id).get();
        let total_supply = self.get_total_lp_token_supply();
        if total_supply != 0 {
            let amount = liquidity * reserve / total_supply;
            self.create_payment(&token_id, 0, &amount)
        } else {
            self.create_payment(&token_id, 0, &BigUint::zero())
        }
    }

    fn get_both_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        let first_token_id = self.first_token_id().get();
        let token_first_token_amount =
            self.get_token_for_given_position(liquidity.clone(), first_token_id);
        let second_token_id = self.second_token_id().get();
        let token_second_token_amount =
            self.get_token_for_given_position(liquidity, second_token_id);
        (token_first_token_amount, token_second_token_amount).into()
    }

    fn calculate_k(&self, context: &dyn Context<Self::Api>) -> BigUint {
        self.calculate_k_constant(
            context.get_first_token_reserve(),
            context.get_second_token_reserve(),
        )
    }

    fn swap_safe_no_fee(
        &self,
        context: &mut dyn Context<Self::Api>,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> BigUint {
        let a_to_b = token_in == context.get_first_token_id();
        match a_to_b {
            true => {
                assert!(
                    self,
                    context.get_first_token_reserve() != &0u64,
                    ERROR_ZERO_AMOUNT,
                );

                let amount_out = self.get_amount_out_no_fee(
                    amount_in,
                    context.get_first_token_reserve(),
                    context.get_second_token_reserve(),
                );
                assert!(
                    self,
                    context.get_second_token_reserve() > &amount_out && amount_out != 0u64,
                    ERROR_ZERO_AMOUNT,
                );

                let new_first_amount = context.get_first_token_reserve() + amount_in;
                let new_second_amount = context.get_second_token_reserve() - &amount_out;
                context.set_first_token_reserve(new_first_amount);
                context.set_second_token_reserve(new_second_amount);

                amount_out
            }
            false => {
                assert!(
                    self,
                    context.get_second_token_reserve() != &0u64,
                    ERROR_ZERO_AMOUNT,
                );

                let amount_out = self.get_amount_out_no_fee(
                    amount_in,
                    context.get_second_token_reserve(),
                    context.get_first_token_reserve(),
                );
                assert!(
                    self,
                    context.get_first_token_reserve() > &amount_out && amount_out != 0u64,
                    ERROR_ZERO_AMOUNT,
                );

                let new_first_amount = context.get_first_token_reserve() - &amount_out;
                let new_second_amount = context.get_second_token_reserve() + amount_in;
                context.set_first_token_reserve(new_first_amount);
                context.set_second_token_reserve(new_second_amount);

                amount_out
            }
        }
    }

    #[view(getTotalSupply)]
    fn get_total_lp_token_supply(&self) -> BigUint {
        self.lp_token_supply().get()
    }

    #[inline]
    fn biguint_min(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a < b {
            a.clone()
        } else {
            b.clone()
        }
    }
}
