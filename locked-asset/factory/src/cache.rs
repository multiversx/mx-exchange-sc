multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{Nonce, UnlockScheduleEx};

use crate::attr_ex_helper;

use super::locked_asset;

#[multiversx_sc::module]
pub trait CacheModule:
    locked_asset::LockedAssetModule + token_send::TokenSendModule + attr_ex_helper::AttrExHelper
{
    #[inline(always)]
    fn get_sft_nonce_for_unlock_schedule(
        &self,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
    ) -> Option<Nonce> {
        self.nonce_cache_ex().get(unlock_schedule)
    }

    #[view(getUnlockScheduleForSFTNonce)]
    fn get_unlock_schedule_for_sft_nonce(
        &self,
        nonce: Nonce,
    ) -> Option<UnlockScheduleEx<Self::Api>> {
        self.unlock_schedule_cache_ex().get(&nonce)
    }

    #[inline(always)]
    fn cache_unlock_schedule_and_nonce(
        &self,
        unlock_schedule: &UnlockScheduleEx<Self::Api>,
        nonce: Nonce,
    ) {
        self.nonce_cache_ex().insert(unlock_schedule.clone(), nonce);
        self.unlock_schedule_cache_ex()
            .insert(nonce, unlock_schedule.clone());
    }

    #[view(getCacheSize)]
    fn get_cache_size(&self) -> usize {
        self.nonce_cache_ex().len()
    }

    #[storage_mapper("nonce_cache_ex")]
    fn nonce_cache_ex(&self) -> MapMapper<UnlockScheduleEx<Self::Api>, Nonce>;

    #[storage_mapper("unlock_schedule_cache_ex")]
    fn unlock_schedule_cache_ex(&self) -> MapMapper<Nonce, UnlockScheduleEx<Self::Api>>;
}
