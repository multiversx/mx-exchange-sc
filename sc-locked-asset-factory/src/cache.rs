elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

use super::locked_asset;
use distrib_common::*;
use modules::*;

const AMOUNT_TO_BURN: u64 = 1;
const GAS_LEFT_THRESHOLD: u64 = 5000000;

#[elrond_wasm_derive::module]
pub trait CacheModule: asset::AssetModule + locked_asset::LockedAssetModule {
    #[inline(always)]
    fn get_cached_sft_nonce_for_attributes(
        &self,
        attributes: &LockedTokenAttributes,
    ) -> Option<Nonce> {
        self.cached_attributes_to_sft_nonce_map().get(attributes)
    }

    #[inline(always)]
    fn cache_attributes_and_nonce(&self, attributes: LockedTokenAttributes, nonce: Nonce) {
        self.cached_attributes_to_sft_nonce_map()
            .insert(attributes, nonce);
    }

    #[endpoint(invalidateCache)]
    fn invalidate_cache(&self, min_nonce: Nonce, max_nonce: Nonce) -> SCResult<u64> {
        only_owner!(self, "Permission denied");
        require!(min_nonce <= max_nonce, "Bad arguments");
        let gas_reserved_for_search = self.blockchain().get_gas_left() / 3;

        let mut to_invalidate = Vec::new();
        for attr in self.cached_attributes_to_sft_nonce_map().keys() {
            if self.blockchain().get_gas_left() < gas_reserved_for_search {
                break;
            }

            let nonce = self
                .cached_attributes_to_sft_nonce_map()
                .get(&attr)
                .unwrap();
            if nonce <= max_nonce && nonce >= min_nonce {
                to_invalidate.push((attr.clone(), nonce));
            }
        }

        let amount_to_burn = Self::BigUint::from(AMOUNT_TO_BURN);
        let token_id = self.locked_asset_token_id().get();

        let mut invalidated_entries = 0;
        for entry in to_invalidate.iter() {
            if self.blockchain().get_gas_left() < GAS_LEFT_THRESHOLD {
                break;
            }

            invalidated_entries += 1;
            self.cached_attributes_to_sft_nonce_map().remove(&entry.0);
            self.send()
                .esdt_nft_burn(&token_id, entry.1, &amount_to_burn);
        }

        Ok(invalidated_entries)
    }

    #[view(getCacheSize)]
    fn get_cache_size(&self) -> usize {
        self.cached_attributes_to_sft_nonce_map().len()
    }

    #[storage_mapper("cached_attributes_to_sft_nonce_map")]
    fn cached_attributes_to_sft_nonce_map(
        &self,
    ) -> MapMapper<Self::Storage, LockedTokenAttributes, Nonce>;
}
