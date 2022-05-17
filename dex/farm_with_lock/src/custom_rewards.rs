elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;

use contexts::generic::StorageCache;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn mint_per_block_rewards(&self) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_block_nonce > last_reward_nonce {
            let to_mint = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

            // Skip the actual minting. Since this SC will deliver locked rewards.

            self.last_reward_block_nonce().set(&current_block_nonce);
            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(&self, storage: &mut StorageCache<Self::Api>) {
        let total_reward = self.mint_per_block_rewards();

        if total_reward > 0u64 {
            storage.reward_reserve += &total_reward;

            if storage.farm_token_supply != 0u64 {
                let increase =
                    total_reward * &storage.division_safety_constant / &storage.farm_token_supply;
                storage.reward_per_share += &increase;
            }
        }
    }

    #[only_owner]
    #[endpoint]
    fn end_produce_rewards(&self) {
        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share().set(&storage.reward_per_share);
        self.reward_reserve().set(&storage.reward_reserve);

        self.produce_rewards_enabled().set(false);
    }

    #[only_owner]
    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share().set(&storage.reward_per_share);
        self.reward_reserve().set(&storage.reward_reserve);

        self.per_block_reward_amount().set(&per_block_amount);
    }
}
