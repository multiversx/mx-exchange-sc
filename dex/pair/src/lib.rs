#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod amm;
pub mod config;
mod contexts;
pub mod errors;
mod events;
pub mod fee;
mod liquidity_pool;
pub mod locking_wrapper;
pub mod pair_actions;
pub mod safe_price;
pub mod safe_price_view;

use crate::errors::*;

use contexts::base::*;
use pair_actions::common_result_types::{
    AddLiquidityResultType, RemoveLiquidityResultType, SwapTokensFixedInputResultType,
    SwapTokensFixedOutputResultType,
};
use pausable::State;
use permissions_module::Permissions;

#[multiversx_sc::contract]
pub trait Pair<ContractReader>:
    amm::AmmModule
    + fee::FeeModule
    + liquidity_pool::LiquidityPoolModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + events::EventsModule
    + safe_price::SafePriceModule
    + safe_price_view::SafePriceViewModule
    + contexts::output_builder::OutputBuilderModule
    + locking_wrapper::LockingWrapperModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + pair_actions::initial_liq::InitialLiquidityModule
    + pair_actions::add_liq::AddLiquidityModule
    + pair_actions::remove_liq::RemoveLiquidityModule
    + pair_actions::swap::SwapModule
    + pair_actions::common_methods::CommonMethodsModule
{
    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: ManagedAddress,
        router_owner_address: ManagedAddress,
        total_fee_percent: u64,
        special_fee_percent: u64,
        initial_liquidity_adder: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        require!(first_token_id.is_valid_esdt_identifier(), ERROR_NOT_AN_ESDT);
        require!(
            second_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(first_token_id != second_token_id, ERROR_SAME_TOKENS);

        let lp_token_id = self.lp_token_identifier().get();
        require!(first_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);
        require!(second_token_id != lp_token_id, ERROR_POOL_TOKEN_IS_PLT);

        self.set_fee_percents(total_fee_percent, special_fee_percent);
        self.state().set(State::Inactive);

        self.router_address().set(&router_address);
        self.first_token_id().set_if_empty(&first_token_id);
        self.second_token_id().set_if_empty(&second_token_id);
        let initial_liquidity_adder_opt = if !initial_liquidity_adder.is_zero() {
            Some(initial_liquidity_adder)
        } else {
            None
        };
        self.initial_liquidity_adder()
            .set_if_empty(&initial_liquidity_adder_opt);

        if admins.is_empty() {
            // backwards compatibility
            let all_permissions = Permissions::OWNER | Permissions::ADMIN | Permissions::PAUSE;
            self.add_permissions(router_address, all_permissions.clone());
            self.add_permissions(router_owner_address, all_permissions);
        } else {
            self.add_permissions(router_address, Permissions::OWNER | Permissions::PAUSE);
            self.add_permissions(
                router_owner_address,
                Permissions::OWNER | Permissions::PAUSE,
            );
            self.add_permissions_for_all(admins, Permissions::ADMIN);
        };
    }

    #[endpoint]
    fn upgrade(&self) {}

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) {
        self.require_caller_has_owner_permissions();

        require!(
            self.lp_token_identifier().is_empty(),
            ERROR_LP_TOKEN_NOT_ISSUED
        );
        require!(
            token_identifier != self.first_token_id().get()
                && token_identifier != self.second_token_id().get(),
            ERROR_LP_TOKEN_SAME_AS_POOL_TOKENS
        );
        require!(
            token_identifier.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        self.lp_token_identifier().set(&token_identifier);
    }

    #[view(getTokensForGivenPosition)]
    fn get_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        self.get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(&self) -> MultiValue3<BigUint, BigUint, BigUint> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let total_supply = self.lp_token_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out_view(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        require!(amount_in > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_in == first_token_id {
            require!(second_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &first_token_reserve, &second_token_reserve);
            require!(second_token_reserve > amount_out, ERROR_NOT_ENOUGH_RESERVE);
            amount_out
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0u64, ERROR_NOT_ENOUGH_RESERVE);
            let amount_out =
                self.get_amount_out(&amount_in, &second_token_reserve, &first_token_reserve);
            require!(first_token_reserve > amount_out, ERROR_NOT_ENOUGH_RESERVE);
            amount_out
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in_view(&self, token_wanted: TokenIdentifier, amount_wanted: BigUint) -> BigUint {
        require!(amount_wanted > 0u64, ERROR_ZERO_AMOUNT);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            require!(
                first_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE
            );

            self.get_amount_in(&amount_wanted, &second_token_reserve, &first_token_reserve)
        } else if token_wanted == second_token_id {
            require!(
                second_token_reserve > amount_wanted,
                ERROR_NOT_ENOUGH_RESERVE
            );

            self.get_amount_in(&amount_wanted, &first_token_reserve, &second_token_reserve)
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }

    #[view(getEquivalent)]
    fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> BigUint {
        require!(amount_in > 0u64, ERROR_ZERO_AMOUNT);
        let zero = BigUint::zero();

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        if first_token_reserve == 0u64 || second_token_reserve == 0u64 {
            return zero;
        }

        if token_in == first_token_id {
            self.quote(&amount_in, &first_token_reserve, &second_token_reserve)
        } else if token_in == second_token_id {
            self.quote(&amount_in, &second_token_reserve, &first_token_reserve)
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        }
    }
}
