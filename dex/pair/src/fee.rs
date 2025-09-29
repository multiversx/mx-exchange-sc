multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::amm;
use super::config;
use super::errors::*;
use super::liquidity_pool;
use crate::contexts::base::StorageCache;
use crate::contexts::base::SwapTokensOrder;

use common_structs::TokenPair;
use fees_collector::fees_accumulation::ProxyTrait as _;
use multiversx_sc::storage::StorageKey;

pub static FEES_COLLECTOR_ADDRESS_STORAGE_KEY: &[u8] = b"fees_collector_address";

mod self_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait PairProxy {
        #[payable("*")]
        #[endpoint(swapNoFeeAndForward)]
        fn swap_no_fee(&self, token_out: TokenIdentifier, destination_address: ManagedAddress);
    }
}

#[multiversx_sc::module]
pub trait FeeModule:
    config::ConfigModule
    + liquidity_pool::LiquidityPoolModule
    + amm::AmmModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    #[endpoint(whitelist)]
    fn whitelist_endpoint(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();
        let is_new = self.whitelist().insert(address);
        require!(is_new, ERROR_ALREADY_WHITELISTED);
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();
        let is_removed = self.whitelist().remove(&address);
        require!(is_removed, ERROR_NOT_WHITELISTED);
    }

    #[endpoint(addTrustedSwapPair)]
    fn add_trusted_swap_pair(
        &self,
        pair_address: ManagedAddress,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_caller_has_owner_permissions();
        require!(first_token != second_token, ERROR_SAME_TOKENS);
        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        let is_new = self
            .trusted_swap_pair()
            .insert(token_pair, pair_address)
            .is_none();
        require!(is_new, ERROR_PAIR_ALREADY_TRUSTED);
    }

    #[endpoint(removeTrustedSwapPair)]
    fn remove_trusted_swap_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_caller_has_owner_permissions();
        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };

        let mut is_removed = self.trusted_swap_pair().remove(&token_pair).is_some();
        if !is_removed {
            let token_pair_reversed = TokenPair {
                first_token: second_token,
                second_token: first_token,
            };
            is_removed = self
                .trusted_swap_pair()
                .remove(&token_pair_reversed)
                .is_some();
            require!(is_removed, ERROR_PAIR_NOT_TRUSTED);
        }
    }

    fn send_fee(&self, fee_token: &TokenIdentifier, fee_amount: &BigUint) {
        if fee_amount == &0u64 {
            return;
        }

        self.send_fees_collector_cut(fee_token.clone(), fee_amount.clone());
    }

    fn send_fees_collector_cut(&self, token: TokenIdentifier, cut_amount: BigUint) {
        let fees_collector_mapper = self.get_fees_collector_address_mapper();
        if fees_collector_mapper.is_empty() {
            return;
        }

        let fees_collector_address = fees_collector_mapper.get();
        let _: IgnoreValue = self
            .fees_collector_proxy(fees_collector_address)
            .deposit_swap_fees()
            .with_esdt_transfer((token, 0, cut_amount))
            .execute_on_dest_context();
    }

    fn send_fee_slice(
        &self,
        storage_cache: &mut StorageCache<Self>,
        swap_tokens_order: SwapTokensOrder,
        fee_token: &TokenIdentifier,
        fee_slice: &BigUint,
        fee_address: &ManagedAddress,
        requested_fee_token: &TokenIdentifier,
    ) {
        let can_send_directly = self.can_send_fee_directly(fee_token, requested_fee_token);
        if can_send_directly {
            self.burn(fee_token, fee_slice);

            return;
        }

        let can_resolve_locally = self.can_resolve_swap_locally(
            fee_token,
            requested_fee_token,
            &storage_cache.first_token_id,
            &storage_cache.second_token_id,
        );
        if can_resolve_locally {
            let to_burn = self.swap_safe_no_fee(storage_cache, swap_tokens_order, fee_slice);
            self.burn(requested_fee_token, &to_burn);

            return;
        }

        let can_extern_swap = self.can_extern_swap_directly(fee_token, requested_fee_token);
        if can_extern_swap {
            self.extern_swap_and_forward(fee_token, fee_slice, requested_fee_token, fee_address);

            return;
        }

        let can_extern_swap_after_local = self.can_extern_swap_after_local_swap(
            &storage_cache.first_token_id,
            &storage_cache.second_token_id,
            fee_token,
            requested_fee_token,
        );
        if can_extern_swap_after_local {
            let to_send = self.swap_safe_no_fee(storage_cache, swap_tokens_order, fee_slice);
            let to_send_token = if fee_token == &storage_cache.first_token_id {
                storage_cache.second_token_id.clone()
            } else {
                storage_cache.first_token_id.clone()
            };

            self.extern_swap_and_forward(
                &to_send_token,
                &to_send,
                requested_fee_token,
                fee_address,
            );
        } else {
            sc_panic!(ERROR_NOTHING_TO_DO_WITH_FEE_SLICE);
        }
    }

    #[inline]
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
    ) {
        let pair_address = self.get_extern_swap_pair_address(available_token, requested_token);

        let _: IgnoreValue = self
            .pair_proxy()
            .contract(pair_address)
            .swap_no_fee(requested_token.clone(), destination_address.clone())
            .with_esdt_transfer((available_token.clone(), 0, available_amount.clone()))
            .execute_on_dest_context();
    }

    #[inline]
    fn burn(&self, token: &TokenIdentifier, amount: &BigUint) {
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
        let is_cached = self
            .trusted_swap_pair()
            .keys()
            .any(|key| key.equals(&token_pair));

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
                .any(|key| key.equals(&token_pair_reversed));

            if is_cached_reversed {
                self.trusted_swap_pair().get(&token_pair_reversed).unwrap()
            } else {
                ManagedAddress::zero()
            }
        }
    }

    #[view(getTrustedSwapPairs)]
    fn get_trusted_swap_pairs(&self) -> MultiValueEncoded<(TokenPair<Self::Api>, ManagedAddress)> {
        let mut result = MultiValueEncoded::new();
        for pair in self.trusted_swap_pair().iter() {
            result.push((pair.0, pair.1))
        }
        result
    }

    #[view(getWhitelistedManagedAddresses)]
    fn get_whitelisted_managed_addresses(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.whitelist().iter() {
            result.push(pair);
        }
        result
    }

    fn get_fees_collector_address_mapper(
        &self,
    ) -> SingleValueMapper<ManagedAddress, ManagedAddress> {
        let router_address = self.router_address().get();
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            router_address,
            StorageKey::new(FEES_COLLECTOR_ADDRESS_STORAGE_KEY),
        )
    }

    #[proxy]
    fn pair_proxy(&self) -> self_proxy::Proxy<Self::Api>;

    #[proxy]
    fn fees_collector_proxy(&self, sc_address: ManagedAddress) -> fees_collector::Proxy<Self::Api>;

    #[storage_mapper("trusted_swap_pair")]
    fn trusted_swap_pair(&self) -> MapMapper<TokenPair<Self::Api>, ManagedAddress>;

    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<ManagedAddress>;
}
