use common_structs::TokenPair;
use fees_collector::fees_accumulation::ProxyTrait as _;

use crate::{
    config::MAX_PERCENTAGE, StorageCache, SwapTokensOrder, ERROR_NOTHING_TO_DO_WITH_FEE_SLICE,
};

multiversx_sc::imports!();

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
pub trait ImplsModule:
    crate::config::ConfigModule
    + crate::liquidity_pool::LiquidityPoolModule
    + crate::amm::AmmModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + super::storage::StorageModule
{
    fn send_fee(
        &self,
        storage_cache: &mut StorageCache<Self>,
        swap_tokens_order: SwapTokensOrder,
        fee_token: &TokenIdentifier,
        fee_amount: &BigUint,
    ) {
        if fee_amount == &0u64 {
            return;
        }

        let fees_collector_configured = !self.fees_collector_address().is_empty();
        let remaining_fee = if fees_collector_configured {
            let fees_collector_cut_percentage = self.fees_collector_cut_percentage().get();
            let cut_amount = fee_amount * fees_collector_cut_percentage / MAX_PERCENTAGE;
            let reminder = fee_amount - &cut_amount;

            if cut_amount > 0 {
                self.send_fees_collector_cut(fee_token.clone(), cut_amount);
            }

            reminder
        } else {
            fee_amount.clone()
        };

        let slices = self.destination_map().len() as u64;
        if slices == 0 {
            return;
        }

        let fee_slice = remaining_fee / slices;
        if fee_slice == 0 {
            return;
        }

        for (fee_address, fee_token_requested) in self.destination_map().iter() {
            self.send_fee_slice(
                storage_cache,
                swap_tokens_order,
                fee_token,
                &fee_slice,
                &fee_address,
                &fee_token_requested,
            );
        }
    }

    fn send_fees_collector_cut(&self, token: TokenIdentifier, cut_amount: BigUint) {
        let fees_collector_address = self.fees_collector_address().get();
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

    #[inline]
    fn burn(&self, token: &TokenIdentifier, amount: &BigUint) {
        if amount > &0 {
            self.send().esdt_local_burn(token, 0, amount);
        }
    }

    #[proxy]
    fn pair_proxy(&self) -> self_proxy::Proxy<Self::Api>;

    #[proxy]
    fn fees_collector_proxy(&self, sc_address: ManagedAddress) -> fees_collector::Proxy<Self::Api>;
}
