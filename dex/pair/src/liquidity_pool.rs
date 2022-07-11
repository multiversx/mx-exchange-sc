elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::base::StorageCache;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::errors::*;

use super::amm;
use super::config;

const MINIMUM_LIQUIDITY: u64 = 1_000;

#[elrond_wasm::module]
pub trait LiquidityPoolModule:
    amm::AmmModule + config::ConfigModule + token_send::TokenSendModule + pausable::PausableModule
{
    fn pool_add_liquidity(
        &self,
        first_token_optimal_amount: &BigUint,
        second_token_optimal_amount: &BigUint,
        storage_cache: &mut StorageCache<Self::Api>,
    ) -> BigUint {
        let first_potential_amt = first_token_optimal_amount * &storage_cache.lp_token_supply
            / &storage_cache.first_token_reserve;
        let second_potential_amt = second_token_optimal_amount * &storage_cache.lp_token_supply
            / &storage_cache.second_token_reserve;

        let liquidity = core::cmp::min(first_potential_amt, second_potential_amt);
        require!(liquidity > 0, ERROR_INSUFFICIENT_LIQUIDITY);

        storage_cache.lp_token_supply += &liquidity;

        storage_cache.first_token_reserve += first_token_optimal_amount;
        storage_cache.second_token_reserve += second_token_optimal_amount;

        liquidity
    }

    fn pool_add_initial_liquidity(
        &self,
        first_token_optimal_amount: &BigUint,
        second_token_optimal_amount: &BigUint,
        storage_cache: &mut StorageCache<Self::Api>,
    ) -> BigUint {
        let liquidity = self.biguint_min(first_token_optimal_amount, second_token_optimal_amount);
        let minimum_liquidity = BigUint::from(MINIMUM_LIQUIDITY);
        require!(liquidity > minimum_liquidity, ERROR_FIRST_LIQUDITY);

        self.send()
            .esdt_local_mint(&storage_cache.lp_token_id, 0, &minimum_liquidity);

        storage_cache.lp_token_supply = liquidity.clone();
        storage_cache.first_token_reserve += first_token_optimal_amount;
        storage_cache.second_token_reserve += second_token_optimal_amount;

        liquidity - minimum_liquidity
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

        require!(
            total_supply >= &(liquidity + MINIMUM_LIQUIDITY),
            ERROR_NOT_ENOUGH_LP
        );

        let first_amount_removed = (liquidity * context.get_first_token_reserve()) / total_supply;
        require!(first_amount_removed > 0u64, ERROR_INSUFFICIENT_LIQ_BURNED);
        require!(
            &first_amount_removed >= context.get_first_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        require!(
            context.get_first_token_reserve() > &first_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let second_amount_removed = (liquidity * context.get_second_token_reserve()) / total_supply;
        require!(second_amount_removed > 0u64, ERROR_INSUFFICIENT_LIQ_BURNED);
        require!(
            &second_amount_removed >= context.get_second_token_amount_min(),
            ERROR_SLIPPAGE_ON_REMOVE
        );
        require!(
            context.get_second_token_reserve() > &second_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        (first_amount_removed, second_amount_removed)
    }

    fn get_initial_liquidity_optimal_amounts(
        &self,
        context: &mut AddLiquidityContext<Self::Api>,
    ) -> (BigUint, BigUint) {
        let first_amount_optimal = context.get_first_payment().amount.clone();
        let second_amount_optimal = context.get_second_payment().amount.clone();
        (first_amount_optimal, second_amount_optimal)
    }

    fn set_optimal_amounts(
        &self,
        context: &mut AddLiquidityContext<Self::Api>,
        storage_cache: &StorageCache<Self::Api>,
    ) {
        let first_token_amount_desired = &context.first_payment.amount;
        let second_token_amount_desired = &context.second_payment.amount;

        let second_token_amount_optimal = self.quote(
            first_token_amount_desired,
            &storage_cache.first_token_reserve,
            &storage_cache.second_token_reserve,
        );

        if &second_token_amount_optimal <= second_token_amount_desired {
            require!(
                second_token_amount_optimal >= context.second_token_amount_min,
                ERROR_INSUFFICIENT_SECOND_TOKEN
            );

            context.first_token_optimal_amount = first_token_amount_desired.clone();
            context.second_token_optimal_amount = second_token_amount_optimal;
        } else {
            let first_token_amount_optimal = self.quote(
                second_token_amount_desired,
                &storage_cache.second_token_reserve,
                &storage_cache.first_token_reserve,
            );
            require!(
                &first_token_amount_optimal <= first_token_amount_desired,
                ERROR_OPTIMAL_GRATER_THAN_PAID
            );
            require!(
                first_token_amount_optimal >= context.first_token_amount_min,
                ERROR_INSUFFICIENT_FIRST_TOKEN
            );

            context.first_token_optimal_amount = first_token_amount_optimal;
            context.second_token_optimal_amount = second_token_amount_desired.clone();
        }
    }

    fn get_token_for_given_position(
        &self,
        liquidity: BigUint,
        token_id: TokenIdentifier,
    ) -> EsdtTokenPayment<Self::Api> {
        let reserve = self.pair_reserve(&token_id).get();
        let total_supply = self.lp_token_supply().get();
        if total_supply != 0 {
            let amount = liquidity * reserve / total_supply;
            EsdtTokenPayment::new(token_id, 0, amount)
        } else {
            EsdtTokenPayment::new(token_id, 0, total_supply)
        }
    }

    fn get_both_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
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
                require!(
                    context.get_first_token_reserve() != &0u64,
                    ERROR_ZERO_AMOUNT
                );

                let amount_out = self.get_amount_out_no_fee(
                    amount_in,
                    context.get_first_token_reserve(),
                    context.get_second_token_reserve(),
                );
                require!(
                    context.get_second_token_reserve() > &amount_out && amount_out != 0u64,
                    ERROR_ZERO_AMOUNT
                );

                let new_first_amount = context.get_first_token_reserve() + amount_in;
                let new_second_amount = context.get_second_token_reserve() - &amount_out;
                context.set_first_token_reserve(new_first_amount);
                context.set_second_token_reserve(new_second_amount);

                amount_out
            }
            false => {
                require!(
                    context.get_second_token_reserve() != &0u64,
                    ERROR_ZERO_AMOUNT
                );

                let amount_out = self.get_amount_out_no_fee(
                    amount_in,
                    context.get_second_token_reserve(),
                    context.get_first_token_reserve(),
                );
                require!(
                    context.get_first_token_reserve() > &amount_out && amount_out != 0u64,
                    ERROR_ZERO_AMOUNT
                );

                let new_first_amount = context.get_first_token_reserve() - &amount_out;
                let new_second_amount = context.get_second_token_reserve() + amount_in;
                context.set_first_token_reserve(new_first_amount);
                context.set_second_token_reserve(new_second_amount);

                amount_out
            }
        }
    }

    fn biguint_min(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a < b {
            a.clone()
        } else {
            b.clone()
        }
    }
}
