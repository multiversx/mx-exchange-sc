elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::amm;
use super::config;
use super::errors::*;
use super::liquidity_pool;
use crate::kill;
use common_structs::TokenPair;

const SWAP_NO_FEE_AND_FORWARD_FUNC_NAME: &[u8] = b"swapNoFeeAndForward";

#[elrond_wasm::module]
pub trait FeeModule:
    config::ConfigModule
    + liquidity_pool::LiquidityPoolModule
    + amm::AmmModule
    + token_send::TokenSendModule
{
    #[storage_mapper("fee_destination")]
    fn destination_map(&self) -> MapMapper<ManagedAddress, TokenIdentifier>;

    #[storage_mapper("trusted_swap_pair")]
    fn trusted_swap_pair(&self) -> MapMapper<TokenPair<Self::Api>, ManagedAddress>;

    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<ManagedAddress>;

    #[view(getFeeState)]
    fn is_fee_enabled(&self) -> bool {
        !self.destination_map().is_empty()
    }

    #[endpoint(whitelist)]
    fn whitelist_endpoint(&self, address: ManagedAddress) {
        self.require_permissions();
        let is_new = self.whitelist().insert(address);
        kill!(self, is_new, ERROR_ALREADY_WHITELISTED);
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: ManagedAddress) {
        self.require_permissions();
        let is_removed = self.whitelist().remove(&address);
        kill!(self, is_removed, ERROR_NOT_WHITELISTED);
    }

    #[endpoint(addTrustedSwapPair)]
    fn add_trusted_swap_pair(
        &self,
        pair_address: ManagedAddress,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_permissions();
        kill!(self, first_token != second_token, ERROR_SAME_TOKENS);
        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        let is_new = self.trusted_swap_pair().insert(token_pair, pair_address) == None;
        kill!(self, is_new, ERROR_PAIR_ALREADY_TRUSTED);
    }

    #[endpoint(removeTrustedSwapPair)]
    fn remove_trusted_swap_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_permissions();
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
            kill!(self, is_removed, ERROR_PAIR_NOT_TRUSTED);
        }
    }

    fn reinject(&self, token: &TokenIdentifier, amount: &BigUint) {
        let mut reserve = self.pair_reserve(token).get();
        reserve += amount;
        self.pair_reserve(token).set(&reserve);
    }

    fn send_fee(&self, fee_token: &TokenIdentifier, fee_amount: &BigUint) {
        if fee_amount == &0 {
            return;
        }

        let slices = self.destination_map().len() as u64;
        if slices == 0 {
            self.reinject(fee_token, fee_amount);
            return;
        }

        let fee_slice = fee_amount / slices;
        if fee_slice == 0 {
            self.reinject(fee_token, fee_amount);
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

        let rounding_error = fee_amount - &(fee_slice * slices);
        if rounding_error > 0 {
            self.reinject(fee_token, &rounding_error);
        }
    }

    fn send_fee_slice(
        &self,
        fee_token: &TokenIdentifier,
        fee_slice: &BigUint,
        fee_address: &ManagedAddress,
        requested_fee_token: &TokenIdentifier,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) {
        if self.can_send_fee_directly(fee_token, requested_fee_token) {
            self.burn_fees(fee_token, fee_slice);
        } else if self.can_resolve_swap_locally(
            fee_token,
            requested_fee_token,
            first_token_id,
            second_token_id,
        ) {
            let to_send =
                self.swap_safe_no_fee(first_token_id, second_token_id, fee_token, fee_slice);
            if to_send > 0 {
                self.burn_fees(requested_fee_token, &to_send);
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
        !pair_address.is_zero()
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
            !pair_address.is_zero()
        } else if fee_token == second_token {
            let pair_address = self.get_extern_swap_pair_address(first_token, requested_fee_token);
            !pair_address.is_zero()
        } else {
            false
        }
    }

    fn extern_swap_and_forward(
        &self,
        available_token: &TokenIdentifier,
        available_amount: &BigUint,
        requested_token: &TokenIdentifier,
        destination_address: &ManagedAddress,
    ) -> bool {
        let pair_address = self.get_extern_swap_pair_address(available_token, requested_token);
        let mut arg_buffer = ManagedArgBuffer::new_empty();
        arg_buffer.push_arg(requested_token);
        arg_buffer.push_arg(destination_address);
        let result = self.raw_vm_api().direct_esdt_execute(
            &pair_address,
            available_token,
            available_amount,
            self.extern_swap_gas_limit().get(),
            &ManagedBuffer::from(SWAP_NO_FEE_AND_FORWARD_FUNC_NAME),
            &arg_buffer,
        );

        match result {
            Result::Ok(()) => true,
            Result::Err(_) => false,
        }
    }

    #[inline]
    fn burn_fees(&self, token: &TokenIdentifier, amount: &BigUint) {
        if amount > &0 {
            self.send().esdt_local_burn(token, 0, amount);
        }
    }

    fn get_extern_swap_pair_address(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) -> ManagedAddress {
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };
        let is_cached = self.trusted_swap_pair().keys().any(|key| {
            key.first_token == token_pair.first_token && key.second_token == token_pair.second_token
        });

        if is_cached {
            self.trusted_swap_pair().get(&token_pair).unwrap()
        } else {
            let token_pair_reversed = TokenPair {
                first_token: second_token.clone(),
                second_token: first_token.clone(),
            };

            let is_cached_reversed = self.trusted_swap_pair().keys().any(|key| {
                key.first_token == token_pair_reversed.first_token
                    && key.second_token == token_pair_reversed.second_token
            });

            if is_cached_reversed {
                self.trusted_swap_pair().get(&token_pair_reversed).unwrap()
            } else {
                ManagedAddress::zero()
            }
        }
    }

    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        enabled: bool,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) {
        self.require_permissions();
        let is_dest = self
            .destination_map()
            .keys()
            .any(|dest_address| dest_address == fee_to_address);

        if enabled {
            kill!(self, !is_dest, ERROR_ALREADY_FEE_DEST);
            self.destination_map().insert(fee_to_address, fee_token);
        } else {
            kill!(self, is_dest, ERROR_NOT_FEE_DEST);
            let dest_fee_token = self.destination_map().get(&fee_to_address).unwrap();
            kill!(self, fee_token == dest_fee_token, ERROR_BAD_TOKEN_FEE_DEST);
            self.destination_map().remove(&fee_to_address);
        }
    }

    #[view(getFeeDestinations)]
    fn get_fee_destinations(&self) -> ManagedMultiResultVec<(ManagedAddress, TokenIdentifier)> {
        let mut result = ManagedMultiResultVec::new();
        for pair in self.destination_map().iter() {
            result.push((pair.0, pair.1))
        }
        result
    }

    #[view(getTrustedSwapPairs)]
    fn get_trusted_swap_pairs(
        &self,
    ) -> ManagedMultiResultVec<(TokenPair<Self::Api>, ManagedAddress)> {
        let mut result = ManagedMultiResultVec::new();
        for pair in self.trusted_swap_pair().iter() {
            result.push((pair.0, pair.1))
        }
        result
    }

    #[view(getWhitelistedManagedAddresses)]
    fn get_whitelisted_managed_addresses(&self) -> ManagedMultiResultVec<ManagedAddress> {
        let mut result = ManagedMultiResultVec::new();
        for pair in self.whitelist().iter() {
            result.push(pair);
        }
        result
    }
}
