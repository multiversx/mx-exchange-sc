elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;
use contexts::storage_cache::StorageCache;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
        let mint_function = |token_id: &TokenIdentifier, amount: &BigUint| {
            self.send().esdt_local_mint(token_id, 0, amount);
        };
        let total_reward =
            self.mint_per_block_rewards(&storage_cache.reward_token_id, mint_function);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }

    #[endpoint]
    fn end_produce_rewards(&self) {
        self.require_caller_is_admin();

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.produce_rewards_enabled().set(false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_caller_is_admin();
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.per_block_reward_amount().set(&per_block_amount);
    }
}
