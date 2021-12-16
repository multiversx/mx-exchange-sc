elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::base::Context;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::die;
use crate::errors::*;

use super::amm;
use super::config;

const MINIMUM_LIQUIDITY: u64 = 1_000;

#[elrond_wasm::module]
pub trait LiquidityPoolModule:
    amm::AmmModule + config::ConfigModule + token_send::TokenSendModule
{
    fn biguint_min(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a < b {
            a.clone()
        } else {
            b.clone()
        }
    }

    fn pool_add_liquidity(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let zero = &BigUint::zero();
        let mut liquidity: BigUint;

        if context.get_lp_token_supply() == zero {
            liquidity = self.biguint_min(
                context.get_first_amount_optimal(),
                context.get_second_amount_optimal(),
            );
            let minimum_liquidity = BigUint::from(MINIMUM_LIQUIDITY);
            die!(self, liquidity > minimum_liquidity, ERROR_FIRST_LIQUDITY);

            liquidity -= &minimum_liquidity;
            let lpt = context.get_lp_token_id();
            self.send().esdt_local_mint(lpt, 0, &minimum_liquidity);
            context.set_lp_token_supply(minimum_liquidity);
        } else {
            let first_payment_amount = context.get_first_amount_optimal();
            let second_payment_amount = context.get_second_amount_optimal();
            let lp_token_supply = context.get_lp_token_supply();
            let first_token_reserve = context.get_first_token_reserve();
            let second_token_reserve = context.get_second_token_reserve();

            liquidity = self.biguint_min(
                &(&(first_payment_amount * lp_token_supply) / first_token_reserve),
                &(&(second_payment_amount * lp_token_supply) / second_token_reserve),
            );
        }

        die!(self, &liquidity > zero, ERROR_INSUFFICIENT_LIQUIDITY);
        context.increase_lp_token_supply(&liquidity);
        context.set_liquidity_added(liquidity);
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

        die!(
            self,
            total_supply >= &(liquidity + MINIMUM_LIQUIDITY),
            ERROR_NOT_ENOUGH_LP
        );

        let first_amount_removed = (liquidity * context.get_first_token_reserve()) / total_supply;
        die!(
            self,
            first_amount_removed > 0u64,
            ERROR_INSUFFICIENT_LIQ_BURNED
        );
        die!(
            self,
            &first_amount_removed >= context.get_first_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        die!(
            self,
            context.get_first_token_reserve() > &first_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let second_amount_removed = (liquidity * context.get_second_token_reserve()) / total_supply;
        die!(
            self,
            second_amount_removed > 0u64,
            ERROR_INSUFFICIENT_LIQ_BURNED
        );
        die!(
            self,
            &second_amount_removed >= context.get_second_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        die!(
            self,
            context.get_second_token_reserve() > &second_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        (first_amount_removed, second_amount_removed)
    }

    fn calculate_optimal_amounts(&self, context: &mut AddLiquidityContext<Self::Api>) {
        let (first_amount_optional, second_amount_optional) = self.get_optiomal_amounts(context);
        context.set_first_amount_optimal(first_amount_optional);
        context.set_second_amount_optimal(second_amount_optional);
    }

    fn get_optiomal_amounts(
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

        if first_token_reserve == zero && second_token_reserve == zero {
            return (
                first_token_amount_desired.clone(),
                second_token_amount_desired.clone(),
            );
        }

        let second_token_amount_optimal = self.quote(
            first_token_amount_desired,
            first_token_reserve,
            second_token_reserve,
        );

        if &second_token_amount_optimal <= second_token_amount_desired {
            die!(
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
            die!(
                self,
                &first_token_amount_optimal <= first_token_amount_desired,
                ERROR_OPTIMAL_GRATER_THAN_PAID,
            );
            die!(
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

    fn update_reserves(
        &self,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) {
        self.pair_reserve(first_token).set(first_token_reserve);
        self.pair_reserve(second_token).set(second_token_reserve);
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

    fn calculate_k_for_reserves(&self) -> BigUint {
        let first_token_amount = self.pair_reserve(&self.first_token_id().get()).get();
        let second_token_amount = self.pair_reserve(&self.second_token_id().get()).get();
        self.calculate_k_constant(&first_token_amount, &second_token_amount)
    }

    fn swap_safe_no_fee(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> BigUint {
        let big_zero = BigUint::zero();
        let first_token_reserve = self.pair_reserve(first_token_id).get();
        let second_token_reserve = self.pair_reserve(second_token_id).get();

        let (token_in, mut reserve_in, token_out, mut reserve_out) = if token_in == first_token_id {
            (
                first_token_id,
                first_token_reserve,
                second_token_id,
                second_token_reserve,
            )
        } else {
            (
                second_token_id,
                second_token_reserve,
                first_token_id,
                first_token_reserve,
            )
        };

        if reserve_out == 0 {
            return big_zero;
        }

        let amount_out = self.get_amount_out_no_fee(amount_in, &reserve_in, &reserve_out);
        if reserve_out <= amount_out || amount_out == 0 {
            return big_zero;
        }

        reserve_in += amount_in;
        reserve_out -= &amount_out;
        self.update_reserves(&reserve_in, &reserve_out, token_in, token_out);

        amount_out
    }

    #[view(getTotalSupply)]
    fn get_total_lp_token_supply(&self) -> BigUint {
        self.lp_token_supply().get()
    }
}
