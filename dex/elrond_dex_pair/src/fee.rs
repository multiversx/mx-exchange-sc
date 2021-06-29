elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::amm;
use super::config;
use super::liquidity_pool;
use common_structs::TokenPair;
use core::iter::FromIterator;

const SWAP_NO_FEE_AND_FORWARD_FUNC_NAME: &[u8] = b"swapNoFeeAndForward";

mod farm_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm_derive::proxy]
    pub trait Farm {
        #[payable("*")]
        #[endpoint]
        fn acceptFee(
            &self,
            #[payment_token] token_in: TokenIdentifier,
            #[payment_amount] amount: Self::BigUint,
        );
    }
}

#[elrond_wasm_derive::module]
pub trait FeeModule:
    config::ConfigModule
    + liquidity_pool::LiquidityPoolModule
    + amm::AmmModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
{
    #[proxy]
    fn farm_proxy(&self, to: Address) -> farm_proxy::Proxy<Self::SendApi>;

    #[storage_mapper("fee_destination")]
    fn destination_map(&self) -> MapMapper<Self::Storage, Address, TokenIdentifier>;

    #[storage_mapper("trusted_swap_pair")]
    fn trusted_swap_pair(&self) -> MapMapper<Self::Storage, TokenPair, Address>;

    #[view(getWhitelistedAddresses)]
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getFeeState)]
    fn is_fee_enabled(&self) -> bool {
        !self.destination_map().is_empty()
    }

    #[endpoint(whitelist)]
    fn whitelist_endpoint(&self, address: Address) -> SCResult<()> {
        self.require_permissions()?;
        let is_new = self.whitelist().insert(address);
        require!(is_new, "Address already whitelisted");
        Ok(())
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: Address) -> SCResult<()> {
        self.require_permissions()?;
        let is_removed = self.whitelist().remove(&address);
        require!(is_removed, "Addresss not whitelisted");
        Ok(())
    }

    #[endpoint(addTrustedSwapPair)]
    fn add_trusted_swap_pair(
        &self,
        pair_address: Address,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> SCResult<()> {
        self.require_permissions()?;
        require!(first_token != second_token, "Tokens should differ");
        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        let is_new = self.trusted_swap_pair().insert(token_pair, pair_address) == None;
        require!(is_new, "Pair already trusted");
        Ok(())
    }

    #[endpoint(removeTrustedSwapPair)]
    fn remove_trusted_swap_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) -> SCResult<()> {
        self.require_permissions()?;
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };

        let mut is_removed = self.trusted_swap_pair().remove(&token_pair) != None;
        if !is_removed {
            let token_pair_reversed = TokenPair {
                first_token: second_token,
                second_token: first_token,
            };
            is_removed = self.trusted_swap_pair().remove(&token_pair_reversed) != None;
            require!(is_removed, "Pair does not exist in trusted pair map");
        }
        Ok(())
    }

    fn reinject(&self, token: &TokenIdentifier, amount: &Self::BigUint) {
        let mut reserve = self.pair_reserve(token).get();
        reserve += amount;
        self.pair_reserve(token).set(&reserve);
    }

    fn send_fee(&self, fee_token: &TokenIdentifier, fee_amount: Self::BigUint) {
        if fee_amount == 0 {
            return;
        }

        let slices = self.destination_map().len() as u64;
        if slices == 0 {
            self.reinject(fee_token, &fee_amount);
            return;
        }

        let fee_slice = &fee_amount / &Self::BigUint::from(slices);
        if fee_slice == 0 {
            self.reinject(fee_token, &fee_amount);
            return;
        }

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        for (fee_address, fee_token_requested) in self.destination_map().iter() {
            self.send_fee_slice(
                fee_token,
                &fee_slice,
                &fee_address,
                &fee_token_requested,
                &first_token_id,
                &second_token_id,
            );
        }

        let rounding_error = fee_amount - fee_slice * Self::BigUint::from(slices);
        if rounding_error > 0 {
            self.reinject(fee_token, &rounding_error);
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
            let first_token_reserve = self.pair_reserve(first_token_id).get();
            let second_token_reserve = self.pair_reserve(second_token_id).get();
            let to_send =
                self.swap_safe_no_fee(first_token_id, second_token_id, fee_token, fee_slice);
            if to_send > 0 {
                let to_send_token = if fee_token == first_token_id {
                    second_token_id
                } else {
                    first_token_id
                };
                let resolved_externally = self.extern_swap_and_forward(
                    to_send_token,
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
        } else {
            self.reinject(fee_token, fee_slice);
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
        let pair_address = self.get_extern_swap_pair_address(fee_token, requested_fee_token);
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
            let pair_address = self.get_extern_swap_pair_address(second_token, requested_fee_token);
            pair_address != Address::zero()
        } else if fee_token == second_token {
            let pair_address = self.get_extern_swap_pair_address(first_token, requested_fee_token);
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
        let pair_address = self.get_extern_swap_pair_address(available_token, requested_token);
        let mut arg_buffer = ArgBuffer::new();
        arg_buffer.push_argument_bytes(requested_token.as_esdt_identifier());
        arg_buffer.push_argument_bytes(destination_address.as_bytes());
        let result = self.send().direct_esdt_execute(
            &pair_address,
            available_token,
            available_amount,
            self.extern_swap_gas_limit().get(),
            SWAP_NO_FEE_AND_FORWARD_FUNC_NAME,
            &arg_buffer,
        );

        match result {
            Result::Ok(()) => true,
            Result::Err(_) => false,
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
                self.burn_tokens(token, amount);
            } else {
                self.farm_proxy(destination.clone())
                    .acceptFee(token.clone(), amount.clone())
                    .execute_on_dest_context();
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

    fn set_fee_on(
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

    #[view(getFeeDestinations)]
    fn get_fee_destinations(&self) -> MultiResultVec<(Address, TokenIdentifier)> {
        MultiResultVec::from_iter(
            self.destination_map()
                .iter()
                .map(|x| (x.0, x.1))
                .collect::<Vec<(Address, TokenIdentifier)>>(),
        )
    }

    #[view(getTrustedSwapPairs)]
    fn get_trusted_swap_pairs(&self) -> MultiResultVec<(TokenPair, Address)> {
        MultiResultVec::from_iter(
            self.trusted_swap_pair()
                .iter()
                .map(|x| (x.0, x.1))
                .collect::<Vec<(TokenPair, Address)>>(),
        )
    }
}
