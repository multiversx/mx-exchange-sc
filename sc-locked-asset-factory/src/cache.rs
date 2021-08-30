elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Nonce, UnlockSchedule};

use super::locked_asset;

#[elrond_wasm::module]
pub trait CacheModule:
    locked_asset::LockedAssetModule + token_supply::TokenSupplyModule + token_send::TokenSendModule
{
    #[inline(always)]
    fn get_sft_nonce_for_unlock_schedule(&self, unlock_schedule: &UnlockSchedule) -> Option<Nonce> {
        self.nonce_cache().get(unlock_schedule)
    }

    #[view(getUnlockScheduleForSFTNonce)]
    fn get_unlock_schedule_for_sft_nonce(&self, nonce: Nonce) -> Option<UnlockSchedule> {
        self.unlock_schedule_cache().get(&nonce)
    }

    #[inline(always)]
    fn cache_unlock_schedule_and_nonce(&self, unlock_schedule: &UnlockSchedule, nonce: Nonce) {
        self.nonce_cache().insert(unlock_schedule.clone(), nonce);
        self.unlock_schedule_cache()
            .insert(nonce, unlock_schedule.clone());
    }

    #[view(getCacheSize)]
    fn get_cache_size(&self) -> usize {
        self.nonce_cache().len()
    }

    #[storage_mapper("nonce_cache")]
    fn nonce_cache(&self) -> SafeMapMapper<Self::Storage, UnlockSchedule, Nonce>;

    #[storage_mapper("unlock_schedule_cache")]
    fn unlock_schedule_cache(&self) -> SafeMapMapper<Self::Storage, Nonce, UnlockSchedule>;
}
