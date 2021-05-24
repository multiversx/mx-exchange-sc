elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;
type Epoch = u64;

use super::locked_asset;
use distrib_common::*;
use modules::*;

const AMOUNT_TO_BURN: u64 = 1;
const GAS_LEFT_THRESHOLD: u64 = 5000000;

#[elrond_wasm_derive::module]
pub trait CacheModule: asset::AssetModule + locked_asset::LockedAssetModule {
    fn get_cached_sft_nonce_for_attributes(
        &self,
        attributes: &LockedTokenAttributes,
    ) -> Option<Nonce> {
        let current_epoch = self.blockchain().get_block_epoch();
        let cache_epoch = self.cache_epoch().get();

        if current_epoch != cache_epoch {
            self.cache_epoch().set(&current_epoch);
            self.cached_attributes_to_sft_nonce_map().clear();
            None
        } else {
            self.cached_attributes_to_sft_nonce_map().get(attributes)
        }
    }

    fn cache_attributes_and_nonce(&self, attributes: LockedTokenAttributes, nonce: Nonce) {
        if self.cached_attributes_to_sft_nonce_map().is_empty() {
            self.first_cached_sft_nonce().set(&nonce);
        }

        self.cached_attributes_to_sft_nonce_map()
            .insert(attributes, nonce);
    }

    #[endpoint(cleanupUnusedTokens)]
    fn cleanup_unused_tokens(&self) -> SCResult<u64> {
        only_owner!(self, "Permission denied");
        let last_burned_sft_nonce_initial = self.last_burned_sft_nonce().get();
        let locked_asset_token_id = self.locked_asset_token_id().get();
        let mut last_burned_sft_nonce = last_burned_sft_nonce_initial;
        let amount_to_burn = Self::BigUint::from(AMOUNT_TO_BURN);
        let current_epoch = self.blockchain().get_block_epoch();
        let cache_epoch = self.cache_epoch().get();

        let limit_nonce_to_burn = if cache_epoch == current_epoch {
            self.first_cached_sft_nonce().get()
        } else {
            self.locked_asset_token_nonce().get() + 1
        };

        for nonce in last_burned_sft_nonce + 1..limit_nonce_to_burn {
            let gas_left = self.blockchain().get_gas_left();

            if gas_left < GAS_LEFT_THRESHOLD {
                break;
            }
            self.burn_locked_assets(&locked_asset_token_id, &amount_to_burn, nonce);
            last_burned_sft_nonce = nonce;
        }

        if last_burned_sft_nonce != last_burned_sft_nonce_initial {
            self.last_burned_sft_nonce().set(&last_burned_sft_nonce);
        }

        Ok(last_burned_sft_nonce - last_burned_sft_nonce_initial)
    }

    #[storage_mapper("cached_attributes_to_sft_nonce_map")]
    fn cached_attributes_to_sft_nonce_map(
        &self,
    ) -> MapMapper<Self::Storage, LockedTokenAttributes, Nonce>;

    #[storage_mapper("cache_epoch")]
    fn cache_epoch(&self) -> SingleValueMapper<Self::Storage, Epoch>;

    #[storage_mapper("last_burned_sft_nonce")]
    fn last_burned_sft_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[storage_mapper("first_cached_sft_nonce")]
    fn first_cached_sft_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;
}
