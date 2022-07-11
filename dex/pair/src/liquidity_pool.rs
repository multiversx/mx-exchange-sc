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

    fn pool_remove_liquidity(
        &self,
        context: &mut RemoveLiquidityContext<Self::Api>,
        storage_cache: &mut StorageCache<Self::Api>,
    ) {
        let (first_amount_removed, second_amount_removed) = self.get_amounts_removed(context);
        storage_cache.lp_token_supply -= &context.lp_token_payment.amount;
        storage_cache.first_token_reserve -= &first_amount_removed;
        storage_cache.second_token_reserve -= &second_amount_removed;

        context.first_token_amount_removed = first_amount_removed;
        context.second_token_amount_removed = second_amount_removed;
    }

    fn get_amounts_removed(
        &self,
        context: &RemoveLiquidityContext<Self::Api>,
        storage_cache: &StorageCache<Self::Api>,
    ) -> (BigUint, BigUint) {
        require!(
            storage_cache.lp_token_supply >= &context.lp_token_payment_amount + MINIMUM_LIQUIDITY,
            ERROR_NOT_ENOUGH_LP
        );

        let first_amount_removed = (&context.lp_token_payment_amount
            * &storage_cache.first_token_reserve)
            / &storage_cache.lp_token_supply;
        require!(first_amount_removed > 0u64, ERROR_INSUFFICIENT_LIQ_BURNED);
        require!(
            first_amount_removed >= context.first_token_amount_min,
            ERROR_SLIPPAGE_ON_REMOVE
        );
        require!(
            storage_cache.first_token_reserve > first_amount_removed,
            ERROR_NOT_ENOUGH_RESERVE
        );

        let second_amount_removed = (&context.lp_token_payment_amount
            * &storage_cache.second_token_reserve)
            / &storage_cache.lp_token_supply;
        require!(second_amount_removed > 0u64, ERROR_INSUFFICIENT_LIQ_BURNED);
        require!(
            second_amount_removed >= context.second_token_amount_min,
            ERROR_SLIPPAGE_ON_REMOVE
        );
        require!(
            storage_cache.second_token_reserve > second_amount_removed,
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

    fn swap_safe_no_fee(
        &self,
        storage_cache: &mut StorageCache<Self::Api>,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> BigUint {
        let (first_token_reserve_ref, second_token_reserve_ref) =
            if token_in == &storage_cache.first_token_id {
                (
                    &mut storage_cache.first_token_reserve,
                    &mut storage_cache.second_token_reserve,
                )
            } else {
                (
                    &mut storage_cache.second_token_reserve,
                    &mut storage_cache.first_token_reserve,
                )
            };

        require!(*first_token_reserve_ref != 0, ERROR_ZERO_AMOUNT);

        let amount_out = self.get_amount_out_no_fee(
            amount_in,
            first_token_reserve_ref,
            &second_token_reserve_ref,
        );
        require!(
            *second_token_reserve_ref > amount_out && amount_out != 0,
            ERROR_ZERO_AMOUNT
        );

        *first_token_reserve_ref += amount_in;
        *second_token_reserve_ref -= &amount_out;

        amount_out
    }

    fn biguint_min(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a < b {
            a.clone()
        } else {
            b.clone()
        }
    }
}
