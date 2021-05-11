#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod amm;
mod fee;
mod liquidity_pool;

use core::cmp::min;
use dex_common::*;

const SWAP_NO_FEE_AND_FORWARD_FUNC_NAME: &[u8] = b"swapNoFeeAndForward";
const EXTERN_SWAP_GAS_LIMIT: u64 = 50000000;
const SEND_FEE_GAS_COST: u64 = 25000000;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
    ActiveNoSwaps,
}

mod farm_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm_derive::proxy]
    pub trait Farm {
        #[payable("*")]
        #[endpoint]
        fn acceptFee(
            &self,
            #[payment_token] token_in: TokenIdentifier,
            #[payment] amount: Self::BigUint,
        );
    }
}

#[elrond_wasm_derive::contract]
pub trait Pair: amm::AmmModule + fee::FeeModule + liquidity_pool::LiquidityPoolModule {
    #[proxy]
    fn farm_proxy(&self, to: Address) -> farm_proxy::Proxy<Self::SendApi>;

    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: Address,
        router_owner_address: Address,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) {
        self.router_address().set(&router_address);
        self.router_owner_address().set(&router_owner_address);
        self.first_token_id().set(&first_token_id);
        self.second_token_id().set(&second_token_id);
        self.total_fee_percent().set(&total_fee_percent);
        self.special_fee_percent().set(&special_fee_percent);
        self.state().set(&State::ActiveNoSwaps);
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Active);
        Ok(())
    }

    #[endpoint(setState)]
    fn set_state(&self, state: State) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&state);
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn acceptEsdtPayment(
        &self,
        #[payment_token] token: TokenIdentifier,
        #[payment] payment: Self::BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            self.call_value().esdt_token_nonce() == 0,
            "Only fungible tokens are accepted in liquidity pools"
        );
        require!(
            payment > 0,
            "PAIR: Funds transfer must be a positive number"
        );
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token == first_token_id || token == second_token_id,
            "Pair: Invalid token"
        );

        let caller = self.blockchain().get_caller();
        let mut temporary_funds = self.temporary_funds(&caller, &token).get();
        temporary_funds += payment;
        self.temporary_funds(&caller, &token).set(&temporary_funds);

        Ok(())
    }

    #[endpoint]
    fn addLiquidity(
        &self,
        first_token_amount_desired: Self::BigUint,
        second_token_amount_desired: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<
        MultiResult3<
            TokenAmountPair<Self::BigUint>,
            TokenAmountPair<Self::BigUint>,
            TokenAmountPair<Self::BigUint>,
        >,
    > {
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

        let caller = self.blockchain().get_caller();
        let old_k = self.calculate_k_for_reserves();
        let expected_first_token_id = self.first_token_id().get();
        let expected_second_token_id = self.second_token_id().get();
        let temporary_first_token_amount = self
            .temporary_funds(&caller, &expected_first_token_id)
            .get();
        let temporary_second_token_amount = self
            .temporary_funds(&caller, &expected_second_token_id)
            .get();

        require!(
            temporary_first_token_amount > 0,
            "Pair: no available first token funds"
        );
        require!(
            temporary_second_token_amount > 0,
            "Pair: no available second token funds"
        );
        require!(
            first_token_amount_desired <= temporary_first_token_amount,
            "Pair: insufficient first token funds to add"
        );
        require!(
            second_token_amount_desired <= temporary_second_token_amount,
            "Pair: insufficient second token funds to add"
        );

        let (first_token_amount, second_token_amount) = self.add_liquidity(
            first_token_amount_desired,
            second_token_amount_desired,
            first_token_amount_min,
            second_token_amount_min,
        )?;

        let lp_token_id = self.lp_token_identifier().get();
        let liquidity = self.mint(
            first_token_amount.clone(),
            second_token_amount.clone(),
            lp_token_id.clone(),
        )?;

        let caller = &self.blockchain().get_caller();
        let temporary_first_token_unused =
            temporary_first_token_amount - first_token_amount.clone();
        let temporary_second_token_unused =
            temporary_second_token_amount - second_token_amount.clone();
        self.temporary_funds(&caller, &expected_first_token_id)
            .clear();
        self.temporary_funds(&caller, &expected_second_token_id)
            .clear();
        self.send_tokens(&lp_token_id, &liquidity, &caller);
        self.send_tokens(
            &expected_first_token_id,
            &temporary_first_token_unused,
            &caller,
        );
        self.send_tokens(
            &expected_second_token_id,
            &temporary_second_token_unused,
            &caller,
        );

        // Once liquidity has been added, the new K should never be lesser than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&old_k, &new_k)?;

        Ok((
            TokenAmountPair {
                token_id: lp_token_id,
                amount: liquidity,
            },
            TokenAmountPair {
                token_id: expected_first_token_id,
                amount: first_token_amount,
            },
            TokenAmountPair {
                token_id: expected_second_token_id,
                amount: second_token_amount,
            },
        )
            .into())
    }

    fn reclaim_temporary_token(&self, caller: &Address, token: &TokenIdentifier) {
        let amount = self.temporary_funds(&caller, token).get();
        self.temporary_funds(&caller, token).clear();
        self.send_tokens(token, &amount, caller);
    }

    #[endpoint(reclaimTemporaryFunds)]
    fn reclaim_temporary_funds(&self) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        let caller = self.blockchain().get_caller();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        self.reclaim_temporary_token(&caller, &first_token_id);
        self.reclaim_temporary_token(&caller, &second_token_id);

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn removeLiquidity(
        &self,
        #[payment_token] liquidity_token: TokenIdentifier,
        #[payment] liquidity: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<MultiResult2<TokenAmountPair<Self::BigUint>, TokenAmountPair<Self::BigUint>>>
    {
        //require!(self.is_active(), "Not active");
        require!(
            !self.lp_token_identifier().is_empty(),
            "LP token not issued"
        );

        let caller = self.blockchain().get_caller();
        let old_k = self.calculate_k_for_reserves();
        let expected_liquidity_token = self.lp_token_identifier().get();
        require!(
            liquidity_token == expected_liquidity_token,
            "Pair: wrong liquidity token"
        );

        let (first_token_amount, second_token_amount) = self.burn(
            liquidity.clone(),
            first_token_amount_min,
            second_token_amount_min,
            self.lp_token_identifier().get(),
        )?;

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        self.send_tokens(&first_token_id, &first_token_amount, &caller);
        self.send_tokens(&second_token_id, &second_token_amount, &caller);

        // Once liquidity has been removed, the new K should never be greater than the old K.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant_strict(&new_k, &old_k)?;

        Ok((
            TokenAmountPair {
                token_id: first_token_id,
                amount: first_token_amount,
            },
            TokenAmountPair {
                token_id: second_token_id,
                amount: second_token_amount,
            },
        )
            .into())
    }

    #[endpoint(whitelist)]
    fn whitelist_endpoint(&self, address: Address) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        self.whitelist().insert(address);
        Ok(())
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: Address) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        self.whitelist().remove(&address);
        Ok(())
    }

    #[endpoint(addTrustedSwapPair)]
    fn add_trusted_swap_pair(
        &self,
        pair_address: Address,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        self.trusted_swap_pair().insert(token_pair, pair_address);
        Ok(())
    }

    #[endpoint(removeTrustedSwapPair)]
    fn remove_trusted_swap_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };
        self.trusted_swap_pair().remove(&token_pair);
        let token_pair_reversed = TokenPair {
            first_token: second_token,
            second_token: first_token,
        };
        self.trusted_swap_pair().remove(&token_pair_reversed);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in: Self::BigUint,
        token_out: TokenIdentifier,
        destination_address: Address,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.whitelist().contains(&caller), "Not whitelisted");
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(token_in != token_out, "Cannot swap same token");
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Invalid token out"
        );

        let old_k = self.calculate_k_for_reserves();

        let amount_out =
            self.swap_safe_no_fee(&first_token_id, &second_token_id, &token_in, &amount_in);
        require!(amount_out > 0, "Zero output");

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant(&old_k, &new_k)?;

        self.send_fee_or_burn_on_zero_address(&token_out, &amount_out, &destination_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out_min: Self::BigUint,
    ) -> SCResult<()> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in > 0, "Invalid amount_in");
        require!(token_in != token_out, "Swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
        );
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out_min,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
        let amount_out_optimal = self.get_amount_out(
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
        require!(amount_out_optimal != 0, "Optimal value is zero");

        let caller = self.blockchain().get_caller();

        let mut fee_amount = Self::BigUint::zero();
        let mut amount_in_after_fee = amount_in.clone();
        if self.is_enabled() {
            fee_amount = self.get_special_fee_from_fixed_input(amount_in);
            amount_in_after_fee -= &fee_amount;
        }

        reserve_token_in += &amount_in_after_fee;
        reserve_token_out -= &amount_out_optimal;

        self.update_reserves(&reserve_token_in, &reserve_token_out, &token_in, &token_out);
        self.send_tokens(&token_out, &amount_out_optimal, &caller);

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_enabled() {
            self.send_fee(token_in, fee_amount);
        }

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant(&old_k, &new_k)?;

        Ok(())
    }

    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount_in_max: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out: Self::BigUint,
    ) -> SCResult<()> {
        require!(self.can_swap(), "Swap is not enabled");
        require!(amount_in_max > 0, "Invalid amount_in");
        require!(token_in != token_out, "Invalid swap with same token");
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        require!(
            token_in == first_token_id || token_in == second_token_id,
            "Pair: Invalid token in"
        );
        require!(
            token_out == first_token_id || token_out == second_token_id,
            "Pair: Invalid token out"
        );
        require!(amount_out != 0, "Desired amount out cannot be zero");
        let old_k = self.calculate_k_for_reserves();

        let mut reserve_token_out = self.pair_reserve(&token_out).get();
        require!(
            reserve_token_out > amount_out,
            "Insufficient reserve for token out"
        );

        let mut reserve_token_in = self.pair_reserve(&token_in).get();
        let amount_in_optimal = self.get_amount_in(
            amount_out.clone(),
            reserve_token_in.clone(),
            reserve_token_out.clone(),
        );
        require!(
            amount_in_optimal <= amount_in_max,
            "Computed amount in greater than maximum amount in"
        );

        let caller = self.blockchain().get_caller();
        let residuum = &amount_in_max - &amount_in_optimal;

        let mut fee_amount = Self::BigUint::zero();
        let mut amount_in_optimal_after_fee = amount_in_optimal.clone();
        if self.is_enabled() {
            fee_amount = self.get_special_fee_from_optimal_input(amount_in_optimal);
            amount_in_optimal_after_fee -= &fee_amount;
        }
        require!(
            reserve_token_out > amount_out,
            "Insufficient amount out reserve"
        );

        reserve_token_in += &amount_in_optimal_after_fee;
        reserve_token_out -= &amount_out;

        self.update_reserves(&reserve_token_in, &reserve_token_out, &token_in, &token_out);
        self.send_tokens(&token_out, &amount_out, &caller);
        self.send_tokens(&token_in, &residuum, &caller);

        //The transaction was made. We are left with $(fee) of $(token_in) as fee.
        if self.is_enabled() {
            self.send_fee(token_in, fee_amount);
        }

        // A swap should not decrease the value of K. Should either be greater or equal.
        let new_k = self.calculate_k_for_reserves();
        self.validate_k_invariant(&old_k, &new_k)?;

        Ok(())
    }

    #[endpoint]
    fn setFeeOn(
        &self,
        enabled: bool,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        let is_dest = self
            .destination_map()
            .keys()
            .any(|dest_address| dest_address == fee_to_address);

        if enabled {
            require!(!is_dest, "Is already a fee destination");
            self.destination_map().insert(fee_to_address, fee_token);
        } else {
            require!(is_dest, "Is not a fee destination");
            let dest_fee_token = self.destination_map().get(&fee_to_address).unwrap();
            require!(fee_token == dest_fee_token, "Destination fee token differs");
            self.destination_map().remove(&fee_to_address);
        }
        Ok(())
    }

    fn reinject(&self, token: &TokenIdentifier, amount: &Self::BigUint) {
        let mut reserve = self.pair_reserve(token).get();
        reserve += amount;
        self.pair_reserve(&token).set(&reserve);
    }

    fn send_fee(&self, fee_token: TokenIdentifier, fee_amount: Self::BigUint) {
        if fee_amount == 0 {
            return;
        }

        let slices = self.destination_map().len() as u64;
        if slices == 0 {
            self.reinject(&fee_token, &fee_amount);
            return;
        }

        let fee_slice = &fee_amount / &Self::BigUint::from(slices);
        if fee_slice == 0 {
            self.reinject(&fee_token, &fee_amount);
            return;
        }

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        for (fee_address, fee_token_requested) in self.destination_map().iter() {
            self.send_fee_slice(
                &fee_token,
                &fee_slice,
                &fee_address,
                &fee_token_requested,
                &first_token_id,
                &second_token_id,
            );
        }
    }

    fn send_fee_slice(
        &self,
        fee_token: &TokenIdentifier,
        fee_slice: &Self::BigUint,
        fee_address: &Address,
        requested_fee_token: &TokenIdentifier,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) {
        if self.can_send_fee_directly(fee_token, requested_fee_token) {
            self.send_fee_or_burn_on_zero_address(fee_token, fee_slice, fee_address);
        } else if self.can_resolve_swap_locally(
            fee_token,
            requested_fee_token,
            first_token_id,
            second_token_id,
        ) {
            let to_send =
                self.swap_safe_no_fee(first_token_id, second_token_id, fee_token, fee_slice);
            if to_send > 0 {
                self.send_fee_or_burn_on_zero_address(requested_fee_token, &to_send, fee_address);
            } else {
                self.reinject(fee_token, fee_slice);
            }
        } else if self.can_extern_swap_directly(fee_token, requested_fee_token) {
            let resolved_externally = self.extern_swap_and_forward(
                fee_token,
                fee_slice,
                requested_fee_token,
                fee_address,
            );
            if !resolved_externally {
                self.reinject(fee_token, fee_slice);
            }
        } else if self.can_extern_swap_after_local_swap(
            first_token_id,
            second_token_id,
            fee_token,
            requested_fee_token,
        ) {
            let to_send =
                self.swap_safe_no_fee(first_token_id, second_token_id, fee_token, fee_slice);
            if to_send > 0 {
                let to_send_token = if fee_token == first_token_id {
                    second_token_id
                } else {
                    first_token_id
                };
                let first_token_reserve = self.pair_reserve(first_token_id).get();
                let second_token_reserve = self.pair_reserve(second_token_id).get();
                let resolved_externally = self.extern_swap_and_forward(
                    &to_send_token,
                    &to_send,
                    requested_fee_token,
                    fee_address,
                );
                if !resolved_externally {
                    //Revert the previous local swap
                    self.update_reserves(
                        &first_token_reserve,
                        &second_token_reserve,
                        first_token_id,
                        second_token_id,
                    );
                    self.reinject(fee_token, fee_slice);
                }
            } else {
                self.reinject(fee_token, fee_slice);
            }
        }
    }

    fn can_send_fee_directly(
        &self,
        fee_token: &TokenIdentifier,
        requested_fee_token: &TokenIdentifier,
    ) -> bool {
        fee_token == requested_fee_token
    }

    fn can_resolve_swap_locally(
        &self,
        fee_token: &TokenIdentifier,
        requested_fee_token: &TokenIdentifier,
        pool_first_token_id: &TokenIdentifier,
        pool_second_token_id: &TokenIdentifier,
    ) -> bool {
        (requested_fee_token == pool_first_token_id && fee_token == pool_second_token_id)
            || (requested_fee_token == pool_second_token_id && fee_token == pool_first_token_id)
    }

    fn can_extern_swap_directly(
        &self,
        fee_token: &TokenIdentifier,
        requested_fee_token: &TokenIdentifier,
    ) -> bool {
        let pair_address = self.get_extern_swap_pair_address(&fee_token, &requested_fee_token);
        pair_address != Address::zero()
    }

    fn can_extern_swap_after_local_swap(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
        fee_token: &TokenIdentifier,
        requested_fee_token: &TokenIdentifier,
    ) -> bool {
        if fee_token == first_token {
            let pair_address =
                self.get_extern_swap_pair_address(&second_token, &requested_fee_token);
            pair_address != Address::zero()
        } else if fee_token == second_token {
            let pair_address =
                self.get_extern_swap_pair_address(&first_token, &requested_fee_token);
            pair_address != Address::zero()
        } else {
            false
        }
    }

    fn extern_swap_and_forward(
        &self,
        available_token: &TokenIdentifier,
        available_amount: &Self::BigUint,
        requested_token: &TokenIdentifier,
        destination_address: &Address,
    ) -> bool {
        let pair_address = self.get_extern_swap_pair_address(&available_token, &requested_token);
        let mut arg_buffer = ArgBuffer::new();
        arg_buffer.push_argument_bytes(requested_token.as_esdt_identifier());
        arg_buffer.push_argument_bytes(destination_address.as_bytes());
        let result = self.send().direct_esdt_execute(
            &pair_address,
            &available_token.as_esdt_identifier(),
            &available_amount,
            min(self.blockchain().get_gas_left(), EXTERN_SWAP_GAS_LIMIT),
            SWAP_NO_FEE_AND_FORWARD_FUNC_NAME,
            &arg_buffer,
        );

        match result {
            Result::Ok(()) => true,
            Result::Err(_) => false,
        }
    }

    #[inline]
    fn send_tokens(&self, token: &TokenIdentifier, amount: &Self::BigUint, destination: &Address) {
        if amount > &0 {
            let _ = self.send().direct_esdt_via_transf_exec(
                destination,
                token.as_esdt_identifier(),
                amount,
                &[],
            );
        }
    }

    #[inline]
    fn send_fee_or_burn_on_zero_address(
        &self,
        token: &TokenIdentifier,
        amount: &Self::BigUint,
        destination: &Address,
    ) {
        if amount > &0 {
            if destination == &Address::zero() {
                self.send().esdt_local_burn(
                    self.blockchain().get_gas_left(),
                    token.as_esdt_identifier(),
                    &amount,
                );
            } else {
                self.farm_proxy(destination.clone())
                    .acceptFee(token.clone(), amount.clone())
                    .execute_on_dest_context(SEND_FEE_GAS_COST);
            }
        }
    }

    fn get_extern_swap_pair_address(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) -> Address {
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };
        let is_cached = self.trusted_swap_pair().keys().any(|key| key == token_pair);

        if is_cached {
            self.trusted_swap_pair().get(&token_pair).unwrap()
        } else {
            let token_pair_reversed = TokenPair {
                first_token: second_token.clone(),
                second_token: first_token.clone(),
            };

            let is_cached_reversed = self
                .trusted_swap_pair()
                .keys()
                .any(|key| key == token_pair_reversed);

            if is_cached_reversed {
                self.trusted_swap_pair().get(&token_pair_reversed).unwrap()
            } else {
                Address::zero()
            }
        }
    }

    #[endpoint]
    fn setLpTokenIdentifier(&self, token_identifier: TokenIdentifier) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.lp_token_identifier().is_empty(), "LP token not empty");
        self.lp_token_identifier().set(&token_identifier);

        Ok(())
    }

    #[inline]
    fn validate_k_invariant(&self, lower: &Self::BigUint, greater: &Self::BigUint) -> SCResult<()> {
        require!(lower <= greater, "K invariant failed");
        Ok(())
    }

    #[inline]
    fn validate_k_invariant_strict(
        &self,
        lower: &Self::BigUint,
        greater: &Self::BigUint,
    ) -> SCResult<()> {
        require!(lower < greater, "K invariant failed");
        Ok(())
    }

    #[view]
    fn getTokensForGivenPosition(
        &self,
        liquidity: Self::BigUint,
    ) -> MultiResult2<TokenAmountPair<Self::BigUint>, TokenAmountPair<Self::BigUint>> {
        self.get_both_tokens_for_given_position(liquidity)
    }

    #[view(getReservesAndTotalSupply)]
    fn get_reserves_and_total_supply(
        &self,
    ) -> MultiResult3<Self::BigUint, Self::BigUint, Self::BigUint> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let total_supply = self.total_supply().get();
        (first_token_reserve, second_token_reserve, total_supply).into()
    }

    #[view(getAmountOut)]
    fn get_amount_out_view(
        &self,
        token_in: TokenIdentifier,
        amount_in: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(amount_in > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_in == first_token_id {
            require!(second_token_reserve > 0, "Zero reserves for second token");
            let amount_out =
                self.get_amount_out(amount_in, first_token_reserve, second_token_reserve.clone());
            require!(
                second_token_reserve > amount_out,
                "Not enough reserves for second token"
            );
            Ok(amount_out)
        } else if token_in == second_token_id {
            require!(first_token_reserve > 0, "Zero reserves for first token");
            let amount_out =
                self.get_amount_out(amount_in, second_token_reserve, first_token_reserve.clone());
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
    fn get_amount_in_view(
        &self,
        token_wanted: TokenIdentifier,
        amount_wanted: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(amount_wanted > 0, "Zero input");

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();

        if token_wanted == first_token_id {
            require!(
                first_token_reserve > amount_wanted,
                "Not enough reserves for first token"
            );
            let amount_in =
                self.get_amount_in(amount_wanted, second_token_reserve, first_token_reserve);
            Ok(amount_in)
        } else if token_wanted == second_token_id {
            require!(
                second_token_reserve > amount_wanted,
                "Not enough reserves for second token"
            );
            let amount_in =
                self.get_amount_in(amount_wanted, first_token_reserve, second_token_reserve);
            Ok(amount_in)
        } else {
            sc_error!("Not a known token")
        }
    }

    #[view]
    fn getEquivalent(
        &self,
        token_in: TokenIdentifier,
        amount_in: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(amount_in > 0, "Zero input");
        let zero = Self::BigUint::zero();

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        if first_token_reserve == 0 || second_token_reserve == 0 {
            return Ok(zero);
        }

        if token_in == first_token_id {
            Ok(self.quote(amount_in, first_token_reserve, second_token_reserve))
        } else if token_in == second_token_id {
            Ok(self.quote(amount_in, second_token_reserve, first_token_reserve))
        } else {
            sc_error!("Not a known token")
        }
    }

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.router_owner_address().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }

    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active || state == State::ActiveNoSwaps
    }

    #[inline]
    fn can_swap(&self) -> bool {
        self.state().get() == State::Active
    }

    #[view]
    fn getLpTokenIdentifier(&self) -> TokenIdentifier {
        self.lp_token_identifier().get()
    }

    #[view(getTemporaryFunds)]
    #[storage_mapper("funds")]
    fn temporary_funds(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;

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
    fn state(&self) -> SingleValueMapper<Self::Storage, State>;
}
