#![no_std]

imports!();
derive_imports!();

pub mod amm;
pub mod fee;
pub mod liquidity_pool;

pub use crate::amm::*;
pub use crate::fee::*;
pub use crate::liquidity_pool::*;

const DEFAULT_TOTAL_FEE_PRECENT: u64 = 3;
const DEFAUL_SPECIAL_FEE_PRECENT: u64 = 1;

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {
    #[module(LiquidityPoolModuleImpl)]
    fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

    #[module(AmmModuleImpl)]
    fn amm(&self) -> AmmModuleImpl<T, BigInt, BigUint>;

    #[module(FeeModuleImpl)]
    fn fee(&self) -> FeeModuleImpl<T, BigInt, BigUint>;

    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: Address,
    ) {
        self.router_address().set(&router_address);
        self.liquidity_pool().first_token_id().set(&first_token_id);
        self.liquidity_pool()
            .second_token_id()
            .set(&second_token_id);
        self.fee().state().set(&false);
        self.amm()
            .total_fee_precent()
            .set(&DEFAULT_TOTAL_FEE_PRECENT);
        self.amm()
            .special_fee_precent()
            .set(&DEFAUL_SPECIAL_FEE_PRECENT);
        self.state().set(&true);
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "permission denied");
        self.state().set(&true);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "permission denied");
        self.state().set(&true);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptEsdtPayment)]
    fn accept_payment_endpoint(
        &self,
        #[payment_token] token: TokenIdentifier,
        #[payment] payment: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            payment > 0,
            "PAIR: Funds transfer must be a positive number"
        );
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        require!(
            token == first_token_id || token == second_token_id,
            "Pair: Invalid token"
        );

        let caller = self.get_caller();
        let mut temporary_funds = self.get_temporary_funds(&caller, &token);
        temporary_funds += payment;
        self.set_temporary_funds(&caller, &token, &temporary_funds);

        Ok(())
    }

    #[endpoint(addLiquidity)]
    fn add_liquidity_endpoint(
        &self,
        first_token_amount_desired: BigUint,
        second_token_amount_desired: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            first_token_amount_desired > 0,
            "Pair: insufficient token a funds sent"
        );
        require!(
            second_token_amount_desired > 0,
            "Pair: insufficient token b funds sent"
        );
        require!(
            !self.lp_token_identifier().is_empty(),
            "Lp token not issued"
        );

        let caller = self.get_caller();
        let old_k = self.liquidity_pool().calculate_k_for_reserves();
        let expected_first_token_id = self.liquidity_pool().first_token_id().get();
        let expected_second_token_id = self.liquidity_pool().second_token_id().get();
        let mut temporary_first_token_amount_desired =
            self.get_temporary_funds(&caller, &expected_first_token_id);
        let mut temporary_second_token_amount_desired =
            self.get_temporary_funds(&caller, &expected_second_token_id);

        require!(
            temporary_first_token_amount_desired > 0,
            "Pair: no available token a funds"
        );
        require!(
            temporary_second_token_amount_desired > 0,
            "Pair: no available token b funds"
        );
        require!(
            first_token_amount_desired <= temporary_first_token_amount_desired,
            "Pair: insufficient token a funds to add"
        );
        require!(
            second_token_amount_desired <= temporary_second_token_amount_desired,
            "Pair: insufficient token b funds to add"
        );

        let (first_token_amount, second_token_amount) =
            sc_try!(self.liquidity_pool().add_liquidity(
                first_token_amount_desired,
                second_token_amount_desired,
                first_token_amount_min,
                second_token_amount_min
            ));

        let lp_token_id = self.lp_token_identifier().get();
        let liquidity = sc_try!(self.liquidity_pool().mint(
            first_token_amount.clone(),
            second_token_amount.clone(),
            lp_token_id.clone(),
        ));

        self.send().direct_esdt_via_transf_exec(
            &self.get_caller(),
            lp_token_id.as_esdt_identifier(),
            &liquidity,
            &[],
        );

        let mut total_supply = self.liquidity_pool().total_supply().get();
        total_supply += liquidity;
        self.liquidity_pool().total_supply().set(&total_supply);

        temporary_first_token_amount_desired -= first_token_amount;
        temporary_second_token_amount_desired -= second_token_amount;
        self.set_temporary_funds(
            &caller,
            &expected_first_token_id,
            &temporary_first_token_amount_desired,
        );
        self.set_temporary_funds(
            &caller,
            &expected_second_token_id,
            &temporary_second_token_amount_desired,
        );

        // Once liquidity has been added, the new K should never be lesser than the old K.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant_strict(&old_k, &new_k));

        Ok(())
    }

    fn reclaim_temporary_token(&self, caller: &Address, token: &TokenIdentifier) {
        let amount = self.get_temporary_funds(&caller, token);
        if amount > 0 {
            self.send().direct_esdt_via_transf_exec(
                &caller,
                token.as_esdt_identifier(),
                &amount,
                &[],
            );
            self.clear_temporary_funds(&caller, token);
        }
    }

    #[endpoint(reclaimTemporaryFunds)]
    fn reclaim_temporary_funds(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        self.reclaim_temporary_token(&caller, &first_token_id);
        self.reclaim_temporary_token(&caller, &&second_token_id);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(
        &self,
        #[payment_token] liquidity_token: TokenIdentifier,
        #[payment] liquidity: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            !self.lp_token_identifier().is_empty(),
            "Lp token not issued"
        );

        let caller = self.get_caller();
        let old_k = self.liquidity_pool().calculate_k_for_reserves();
        let expected_liquidity_token = self.lp_token_identifier().get();
        require!(
            liquidity_token == expected_liquidity_token,
            "Pair: wrong liquidity token"
        );

        let (first_token_amount, second_token_amount) = sc_try!(self.liquidity_pool().burn(
            liquidity.clone(),
            first_token_amount_min,
            second_token_amount_min,
            self.lp_token_identifier().get(),
        ));

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let mut total_supply = self.liquidity_pool().total_supply().get();
        require!(total_supply > liquidity, "Not enough supply");
        total_supply -= liquidity;

        self.send().direct_esdt_via_transf_exec(
            &caller,
            first_token_id.as_esdt_identifier(),
            &first_token_amount,
            &[],
        );
        self.send().direct_esdt_via_transf_exec(
            &caller,
            second_token_id.as_esdt_identifier(),
            &second_token_amount,
            &[],
        );
        self.liquidity_pool().total_supply().set(&total_supply);

        // Once liquidity has been removed, the new K should never be greater than the old K.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant_strict(&new_k, &old_k));

        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in: BigUint,
        token_out: TokenIdentifier,
        amount_out_min: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(amount_in > 0, "Invalid amount_in");
        require!(token_in != token_out, "Swap with same token");
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token"
        );
        let old_k = self.liquidity_pool().calculate_k_for_reserves();

        let mut reserve_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
        require!(
            reserve_token_out > amount_out_min,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
        let amount_out_optimal = self.amm().get_amount_out(
            amount_in.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        require!(
            amount_out_optimal >= amount_out_min,
            "Insufficient liquidity"
        );
        require!(
            reserve_token_out > amount_out_optimal,
            "Insufficient reserve"
        );

        self.send().direct_esdt_via_transf_exec(
            &self.get_caller(),
            token_out.as_esdt_identifier(),
            &amount_out_optimal,
            &[],
        );

        let mut fee_amount = BigUint::zero();
        let mut amount_in_after_fee = amount_in.clone();
        if self.fee().is_enabled() {
            fee_amount = self.amm().get_special_fee_from_fixed_input(amount_in);
            amount_in_after_fee -= fee_amount.clone();
        }

        reserve_token_in += amount_in_after_fee;
        reserve_token_out -= amount_out_optimal;

        self.liquidity_pool().update_reserves(
            &reserve_token_in,
            &reserve_token_out,
            &token_in,
            &token_out,
        );

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.fee().is_enabled() {
            self.send_fee(token_in, fee_amount);
        }

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant(&old_k, &new_k));

        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in_max: BigUint,
        token_out: TokenIdentifier,
        amount_out: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(amount_in_max > 0, "Invalid amount_in");
        require!(token_in != token_out, "Swap with same token");
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token"
        );
        let old_k = self.liquidity_pool().calculate_k_for_reserves();

        let mut reserve_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
        require!(
            reserve_token_out > amount_out,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
        let amount_in_optimal = self.amm().get_amount_in(
            amount_out.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        require!(amount_in_optimal <= amount_in_max, "Insufficient liquidity");

        self.send().direct_esdt_via_transf_exec(
            &self.get_caller(),
            token_out.as_esdt_identifier(),
            &amount_out,
            &[],
        );
        let residuum = amount_in_max - amount_in_optimal.clone();
        if residuum != BigUint::from(0u64) {
            self.send().direct_esdt_via_transf_exec(
                &self.get_caller(),
                token_in.as_esdt_identifier(),
                &residuum,
                &[],
            );
        }

        let mut fee_amount = BigUint::zero();
        let mut amount_in_optimal_after_fee = amount_in_optimal.clone();
        if self.fee().is_enabled() {
            fee_amount = self
                .amm()
                .get_special_fee_from_optimal_input(amount_in_optimal);
            amount_in_optimal_after_fee -= fee_amount.clone();
        }
        require!(reserve_token_out > amount_out, "Insufficient reserve");

        reserve_token_in += amount_in_optimal_after_fee;
        reserve_token_out -= amount_out;

        self.liquidity_pool().update_reserves(
            &reserve_token_in,
            &reserve_token_out,
            &token_in,
            &token_out,
        );

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.fee().is_enabled() {
            self.send_fee(token_in, fee_amount);
        }

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant(&old_k, &new_k));

        Ok(())
    }

    #[endpoint]
    fn set_fee_on(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "permission denied");
        self.fee().state().set(&enabled);
        self.fee().address().set(&fee_to_address);
        self.fee().token_identifier().set(&fee_token);
        Ok(())
    }

    fn send_fee(&self, fee_token: TokenIdentifier, fee_amount: BigUint) {
        if fee_amount == BigUint::zero() {
            return;
        }

        let fee_token_requested = self.fee().token_identifier().get();
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let mut to_send = BigUint::zero();

        if fee_token_requested == fee_token {
            // Luckily no conversion is required.
            to_send = fee_amount.clone();
        } else if fee_token_requested == first_token_id && fee_token == second_token_id {
            // Fees are in form of second_token_id.  Need to convert them to first_token_id.
            let mut second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);
            let mut first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
            let fee_amount_swap = self.amm().get_amount_out_no_fee(
                fee_amount.clone(),
                second_token_reserve.clone(),
                first_token_reserve.clone(),
            );

            if first_token_reserve > fee_amount_swap && fee_amount_swap > BigUint::zero() {
                //There are enough tokens for swapping.
                first_token_reserve -= fee_amount_swap.clone();
                second_token_reserve += fee_amount.clone();
                to_send = fee_amount_swap;

                self.liquidity_pool().update_reserves(
                    &first_token_reserve,
                    &second_token_reserve,
                    &first_token_id,
                    &second_token_id,
                );
            }
        } else if fee_token_requested == second_token_id && fee_token == first_token_id {
            // Fees are in form of first_token_id.  Need to convert them to second_token_id.
            let mut first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
            let mut second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);
            let fee_amount_swap = self.amm().get_amount_out_no_fee(
                fee_amount.clone(),
                first_token_reserve.clone(),
                second_token_reserve.clone(),
            );

            if second_token_reserve > fee_amount_swap && fee_amount_swap > BigUint::zero() {
                second_token_reserve -= fee_amount_swap.clone();
                first_token_reserve += fee_amount.clone();
                //There are enough tokens for swapping.
                to_send = fee_amount_swap;

                self.liquidity_pool().update_reserves(
                    &first_token_reserve,
                    &second_token_reserve,
                    &first_token_id,
                    &second_token_id,
                );
            }
        }

        if to_send > BigUint::zero() {
            self.send().direct_esdt_via_transf_exec(
                &self.fee().address().get(),
                self.fee().token_identifier().get().as_esdt_identifier(),
                &to_send,
                &[],
            );
        } else {
            // Either swap failed or requested token identifier differs from both first_token_id and second_token_id.
            // Reinject them into liquidity pool.
            let mut reserve = self.liquidity_pool().get_pair_reserve(&fee_token);
            reserve += fee_amount;
            self.liquidity_pool().set_pair_reserve(&fee_token, &reserve);
        }
    }

    #[endpoint]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "permission denied");
        require!(self.lp_token_identifier().is_empty(), "Lp token not empty");
        self.lp_token_identifier().set(&token_identifier);

        Ok(())
    }

    fn validate_k_invariant(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower <= greater, "K invariant failed");
        Ok(())
    }

    fn validate_k_invariant_strict(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower < greater, "K invariant failed");
        Ok(())
    }

    #[view]
    fn get_lp_token_identifier(&self) -> TokenIdentifier {
        self.lp_token_identifier().get()
    }

    #[view]
    fn get_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>> {
        self.liquidity_pool()
            .get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(&self) -> MultiResult3<BigUint, BigUint, BigUint> {
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
        let second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);
        let total_supply = self.liquidity_pool().total_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out(&self, token_in: TokenIdentifier, amount_in: BigUint) -> SCResult<BigUint> {
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
        let second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);

        if token_in == first_token_id {
            require!(second_token_reserve > 0, "Zero reserves");
            let amount_out = self.amm().get_amount_out(
                amount_in,
                first_token_reserve,
                second_token_reserve.clone(),
            );
            require!(second_token_reserve > amount_out, "Not enough reserves");
            Ok(amount_out)
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0, "Zero reserves");
            let amount_out = self.amm().get_amount_out(
                amount_in,
                second_token_reserve,
                first_token_reserve.clone(),
            );
            require!(first_token_reserve > amount_out, "Not enough reserves");
            Ok(amount_out)
        } else {
            sc_error!("Not a known token")
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in(
        &self,
        token_wanted: TokenIdentifier,
        amount_wanted: BigUint,
    ) -> SCResult<BigUint> {
        require!(amount_wanted > 0, "Zero input");

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
        let second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);

        if token_wanted == first_token_id {
            require!(first_token_reserve > amount_wanted, "Not enough reserves");
            let amount_in =
                self.amm()
                    .get_amount_in(amount_wanted, second_token_reserve, first_token_reserve);
            Ok(amount_in)
        } else if token_wanted == second_token_id {
            require!(second_token_reserve > amount_wanted, "Not enough reserves");
            let amount_in =
                self.amm()
                    .get_amount_in(amount_wanted, first_token_reserve, second_token_reserve);
            Ok(amount_in)
        } else {
            sc_error!("Not a known token")
        }
    }

    #[view(getEquivalent)]
    fn get_equivalent(&self, token_in: TokenIdentifier, amount_in: BigUint) -> SCResult<BigUint> {
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let first_token_reserve = self.liquidity_pool().get_pair_reserve(&first_token_id);
        let second_token_reserve = self.liquidity_pool().get_pair_reserve(&second_token_id);
        require!(first_token_reserve > 0, "Not enough reserves");
        require!(second_token_reserve > 0, "Not enough reserves");

        if token_in == first_token_id {
            Ok(self
                .amm()
                .quote(amount_in, first_token_reserve, second_token_reserve))
        } else if token_in == second_token_id {
            Ok(self
                .amm()
                .quote(amount_in, second_token_reserve, first_token_reserve))
        } else {
            sc_error!("Not a known token")
        }
    }

    fn is_active(&self) -> bool {
        self.state().get()
    }

    // Temporary Storage
    #[view(getTemporaryFunds)]
    #[storage_get("funds")]
    fn get_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier) -> BigUint;

    #[storage_set("funds")]
    fn set_temporary_funds(
        &self,
        caller: &Address,
        token_identifier: &TokenIdentifier,
        amount: &BigUint,
    );

    #[storage_clear("funds")]
    fn clear_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier);

    #[storage_mapper("lpTokenIdentifier")]
    fn lp_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getRouterAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;
}
