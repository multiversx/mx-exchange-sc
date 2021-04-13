#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod amm;
pub mod fee;
pub mod liquidity_pool;

use core::cmp::min;

pub use crate::amm::*;
pub use crate::fee::*;
pub use crate::liquidity_pool::*;

const SWAP_NO_FEE_FUNC_NAME: &[u8] = b"swapNoFee";
const EXTERN_SWAP_GAS_LIMIT: u64 = 25000000;

#[elrond_wasm_derive::callable(RouterContractProxy)]
pub trait RouterContract {
    fn getPairAndWhitelist(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> ContractCall<BigUint, Address>;
}

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
        router_owner_address: Address,
        total_fee_precent: u64,
        special_fee_precent: u64,
    ) {
        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.liquidity_pool().first_token_id().set(&first_token_id);
        self.liquidity_pool()
            .second_token_id()
            .set(&second_token_id);
        self.amm().total_fee_precent().set(&total_fee_precent);
        self.amm().special_fee_precent().set(&special_fee_precent);
        self.state().set(&true);
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        let router_owner = self.router_owner_address().get();
        require!(
            caller == router || caller == router_owner,
            "permission denied"
        );
        self.state().set(&true);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        let router_owner = self.router_owner_address().get();
        require!(
            caller == router || caller == router_owner,
            "permission denied"
        );
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
        let mut temporary_funds = self.temporary_funds(&caller, &token).get();
        temporary_funds += payment;
        self.temporary_funds(&caller, &token).set(&temporary_funds);

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
            "Pair: insufficient first token funds sent"
        );
        require!(
            second_token_amount_desired > 0,
            "Pair: insufficient second token funds sent"
        );
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );

        let caller = self.get_caller();
        let old_k = self.liquidity_pool().calculate_k_for_reserves();
        let expected_first_token_id = self.liquidity_pool().first_token_id().get();
        let expected_second_token_id = self.liquidity_pool().second_token_id().get();
        let mut temporary_first_token_amount_desired = self
            .temporary_funds(&caller, &expected_first_token_id)
            .get();
        let mut temporary_second_token_amount_desired = self
            .temporary_funds(&caller, &expected_second_token_id)
            .get();

        require!(
            temporary_first_token_amount_desired > 0,
            "Pair: no available first token funds"
        );
        require!(
            temporary_second_token_amount_desired > 0,
            "Pair: no available second token funds"
        );
        require!(
            first_token_amount_desired <= temporary_first_token_amount_desired,
            "Pair: insufficient first token funds to add"
        );
        require!(
            second_token_amount_desired <= temporary_second_token_amount_desired,
            "Pair: insufficient second token funds to add"
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
        self.temporary_funds(&caller, &expected_first_token_id)
            .set(&temporary_first_token_amount_desired);
        self.temporary_funds(&caller, &expected_second_token_id)
            .set(&temporary_second_token_amount_desired);

        // Once liquidity has been added, the new K should never be lesser than the old K.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant_strict(&old_k, &new_k));

        Ok(())
    }

    fn reclaim_temporary_token(&self, caller: &Address, token: &TokenIdentifier) {
        let amount = self.temporary_funds(&caller, token).get();
        if amount > 0 {
            self.send().direct_esdt_via_transf_exec(
                &caller,
                token.as_esdt_identifier(),
                &amount,
                &[],
            );
            self.temporary_funds(&caller, token).clear();
        }
    }

    #[endpoint(reclaimTemporaryFunds)]
    fn reclaim_temporary_funds(&self) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        self.reclaim_temporary_token(&caller, &first_token_id);
        self.reclaim_temporary_token(&caller, &second_token_id);

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
        //require!(self.is_active(), "Not active");
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
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

    #[endpoint(whitelist)]
    fn whitelist(&self, address: Address) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        let router_owner = self.router_owner_address().get();
        require!(
            caller == router || caller == router_owner,
            "permission denied"
        );
        self.fee().whitelist().insert(address);
        Ok(())
    }

    #[endpoint(cachePair)]
    fn cache_pair(
        &self,
        pair_address: Address,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        let router_owner = self.router_owner_address().get();
        require!(
            caller == router || caller == router_owner,
            "permission denied"
        );
        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        self.fee()
            .pair_address_cache_map()
            .insert(token_pair, pair_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapNoFee)]
    fn swap_no_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in: BigUint,
    ) -> SCResult<()> {
        let caller = self.get_caller();
        require!(self.fee().whitelist().contains(&caller), "Not whitelisted");

        if !self.is_active() {
            self.send().direct_esdt_via_transf_exec(
                &caller,
                token_in.as_esdt_identifier(),
                &amount_in,
                &[],
            );
            return Ok(());
        }
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let token_out = if token_in == first_token_id {
            second_token_id
        } else if token_in == second_token_id {
            first_token_id
        } else {
            return sc_error!("Bad token input");
        };
        let old_k = self.liquidity_pool().calculate_k_for_reserves();

        let mut reserve_token_out = self.liquidity_pool().pair_reserve(&token_out).get();
        if reserve_token_out == 0 {
            self.send().direct_esdt_via_transf_exec(
                &caller,
                token_in.as_esdt_identifier(),
                &amount_in,
                &[],
            );
            return Ok(());
        }

        let mut reserve_token_in = self.liquidity_pool().pair_reserve(&token_in).get();
        let amount_out = self.amm().get_amount_out_no_fee(
            amount_in.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        if amount_out == 0 {
            self.send().direct_esdt_via_transf_exec(
                &caller,
                token_in.as_esdt_identifier(),
                &amount_in,
                &[],
            );
            return Ok(());
        }

        reserve_token_in += amount_in;
        reserve_token_out -= amount_out.clone();

        self.liquidity_pool().update_reserves(
            &reserve_token_in,
            &reserve_token_out,
            &token_in,
            &token_out,
        );

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.liquidity_pool().calculate_k_for_reserves();
        sc_try!(self.validate_k_invariant(&old_k, &new_k));

        self.send().direct_esdt_via_transf_exec(
            &caller,
            token_out.as_esdt_identifier(),
            &amount_out,
            &[],
        );
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
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
        );
        let old_k = self.liquidity_pool().calculate_k_for_reserves();

        let mut reserve_token_out = self.liquidity_pool().pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out_min,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.liquidity_pool().pair_reserve(&token_in).get();
        let amount_out_optimal = self.amm().get_amount_out(
            amount_in.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        require!(
            amount_out_optimal >= amount_out_min,
            "Computed amount out lesser than minimum amount out"
        );
        require!(
            reserve_token_out > amount_out_optimal,
            "Insufficient amount out reserve"
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
        require!(token_in != token_out, "Invalid swap with same token");
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
        );
        let old_k = self.liquidity_pool().calculate_k_for_reserves();

        let mut reserve_token_out = self.liquidity_pool().pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.liquidity_pool().pair_reserve(&token_in).get();
        let amount_in_optimal = self.amm().get_amount_in(
            amount_out.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        require!(
            amount_in_optimal <= amount_in_max,
            "Computed amount in grater than maximum amount in"
        );

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
        require!(
            reserve_token_out > amount_out,
            "Insufficient amount out reserve"
        );

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

    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "Permission denied");
        let is_dest = self
            .fee()
            .destination_map()
            .keys()
            .any(|dest_address| dest_address == fee_to_address);

        if enabled {
            require!(!is_dest, "Is already a fee destination");
            self.fee()
                .destination_map()
                .insert(fee_to_address, fee_token);
        } else {
            require!(is_dest, "Is not a fee destination");
            let dest_fee_token = self.fee().destination_map().get(&fee_to_address).unwrap();
            require!(fee_token == dest_fee_token, "Destination fee token differs");
            self.fee().destination_map().remove(&fee_to_address);
        }
        Ok(())
    }

    fn reinject(&self, token: &TokenIdentifier, amount: &BigUint) {
        let mut reserve = self.liquidity_pool().pair_reserve(token).get();
        reserve += amount;
        self.liquidity_pool().pair_reserve(&token).set(&reserve);
    }

    fn send_fee(&self, fee_token: TokenIdentifier, fee_amount: BigUint) {
        if fee_amount == 0 {
            return;
        }

        let slices = self.fee().destination_map().len() as u64;
        if slices == 0 {
            self.reinject(&fee_token, &fee_amount);
            return;
        }
        let fee_slice = fee_amount.clone() / BigUint::from(slices);
        if fee_slice == 0 {
            self.reinject(&fee_token, &fee_amount);
            return;
        }
        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();

        for (fee_address, fee_token_requested) in self.fee().destination_map().iter() {
            let mut to_send = BigUint::zero();
            if fee_token_requested == fee_token {
                // Luckily no conversion is required.
                to_send = fee_slice.clone();
            } else if fee_token_requested == first_token_id && fee_token == second_token_id {
                // Fees are in form of second_token_id.  Need to convert them to first_token_id.
                let mut second_token_reserve =
                    self.liquidity_pool().pair_reserve(&second_token_id).get();
                let mut first_token_reserve =
                    self.liquidity_pool().pair_reserve(&first_token_id).get();
                let fee_amount_swap = self.amm().get_amount_out_no_fee(
                    fee_slice.clone(),
                    second_token_reserve.clone(),
                    first_token_reserve.clone(),
                );

                if first_token_reserve > fee_amount_swap && fee_amount_swap > BigUint::zero() {
                    //There are enough tokens for swapping.
                    first_token_reserve -= fee_amount_swap.clone();
                    second_token_reserve += fee_slice.clone();
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
                let mut first_token_reserve =
                    self.liquidity_pool().pair_reserve(&first_token_id).get();
                let mut second_token_reserve =
                    self.liquidity_pool().pair_reserve(&second_token_id).get();
                let fee_amount_swap = self.amm().get_amount_out_no_fee(
                    fee_slice.clone(),
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
            } else {
                // No luck... The hard way
                let pair_address = self.get_pair_address(&fee_token, &fee_token_requested);

                if pair_address != Address::zero() {
                    self.send().direct_esdt_execute(
                        &pair_address,
                        &fee_token.as_esdt_identifier(),
                        &fee_slice,
                        min(self.get_gas_left(), EXTERN_SWAP_GAS_LIMIT),
                        SWAP_NO_FEE_FUNC_NAME,
                        &ArgBuffer::new(),
                    );

                    to_send = self.get_esdt_balance(
                        &self.get_sc_address(),
                        fee_token_requested.as_esdt_identifier(),
                        0,
                    );
                }
            }

            if to_send > 0 {
                self.send().direct_esdt_via_transf_exec(
                    &fee_address,
                    &fee_token_requested.as_esdt_identifier(),
                    &to_send,
                    &[],
                );
            } else {
                self.reinject(&fee_token, &fee_slice);
            }
        }
    }

    fn get_pair_address(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) -> Address {
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };
        let token_pair_reversed = TokenPair {
            first_token: second_token.clone(),
            second_token: first_token.clone(),
        };
        let is_cached = self
            .fee()
            .pair_address_cache_map()
            .keys()
            .any(|key| key == token_pair);
        let is_cached_reversed = self
            .fee()
            .pair_address_cache_map()
            .keys()
            .any(|key| key == token_pair_reversed);

        if is_cached {
            self.fee()
                .pair_address_cache_map()
                .get(&token_pair)
                .unwrap()
        } else if is_cached_reversed {
            self.fee()
                .pair_address_cache_map()
                .get(&token_pair_reversed)
                .unwrap()
        } else {
            Address::zero()
        }
    }

    #[endpoint(setLpTokenIdentifier)]
    fn set_lp_token_identifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        let router_owner = self.router_owner_address().get();
        require!(
            caller == router || caller == router_owner,
            "permission denied"
        );
        require!(self.lp_token_identifier().is_empty(), "LP token not empty");
        self.lp_token_identifier().set(&token_identifier);

        Ok(())
    }

    #[inline]
    fn validate_k_invariant(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower <= greater, "K invariant failed");
        Ok(())
    }

    #[inline]
    fn validate_k_invariant_strict(&self, lower: &BigUint, greater: &BigUint) -> SCResult<()> {
        require!(lower < greater, "K invariant failed");
        Ok(())
    }

    #[view(getTokensForGivenPosition)]
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
        let first_token_reserve = self.liquidity_pool().pair_reserve(&first_token_id).get();
        let second_token_reserve = self.liquidity_pool().pair_reserve(&second_token_id).get();
        let total_supply = self.liquidity_pool().total_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out(&self, token_in: TokenIdentifier, amount_in: BigUint) -> SCResult<BigUint> {
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.liquidity_pool().first_token_id().get();
        let second_token_id = self.liquidity_pool().second_token_id().get();
        let first_token_reserve = self.liquidity_pool().pair_reserve(&first_token_id).get();
        let second_token_reserve = self.liquidity_pool().pair_reserve(&second_token_id).get();

        if token_in == first_token_id {
            require!(second_token_reserve > 0, "Zero reserves for second token");
            let amount_out = self.amm().get_amount_out(
                amount_in,
                first_token_reserve,
                second_token_reserve.clone(),
            );
            require!(
                second_token_reserve > amount_out,
                "Not enough reserves for second token"
            );
            Ok(amount_out)
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0, "Zero reserves for first token");
            let amount_out = self.amm().get_amount_out(
                amount_in,
                second_token_reserve,
                first_token_reserve.clone(),
            );
            require!(
                first_token_reserve > amount_out,
                "Not enough reserves first token"
            );
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
        let first_token_reserve = self.liquidity_pool().pair_reserve(&first_token_id).get();
        let second_token_reserve = self.liquidity_pool().pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            require!(
                first_token_reserve > amount_wanted,
                "Not enough reserves for first token"
            );
            let amount_in =
                self.amm()
                    .get_amount_in(amount_wanted, second_token_reserve, first_token_reserve);
            Ok(amount_in)
        } else if token_wanted == second_token_id {
            require!(
                second_token_reserve > amount_wanted,
                "Not enough reserves for second token"
            );
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
        let first_token_reserve = self.liquidity_pool().pair_reserve(&first_token_id).get();
        let second_token_reserve = self.liquidity_pool().pair_reserve(&second_token_id).get();
        require!(
            first_token_reserve > 0,
            "Not enough reserves for first token"
        );
        require!(
            second_token_reserve > 0,
            "Not enough reserves for second token"
        );

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

    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[view(getTemporaryFunds)]
    #[storage_mapper("funds")]
    fn temporary_funds(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, BigUint>;

    #[view(getLpTokenIdentifier)]
    #[storage_mapper("lpTokenIdentifier")]
    fn lp_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getRouterAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getRouterOwnerAddress)]
    #[storage_mapper("router_owner_address")]
    fn router_owner_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;
}
