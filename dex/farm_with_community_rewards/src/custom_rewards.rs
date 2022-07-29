elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use community_rewards::MINIMUM_REWARDING_BLOCKS;
use contexts::generic::StorageCache;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
    + community_rewards::CommunityRewardsModule
    + pausable::PausableModule
    + elrond_wasm_modules::only_admin::OnlyAdminModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn distribute_per_block_rewards(&self) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_block_nonce > last_reward_nonce {
            let mut to_distribute =
                self.calculate_per_block_community_rewards(current_block_nonce, last_reward_nonce);

            if to_distribute != 0 {
                let community_rewards_remaining_reserve_mapper =
                    self.community_rewards_remaining_reserve();
                let community_rewards_remaining_reserve =
                    community_rewards_remaining_reserve_mapper.get();

                if to_distribute >= community_rewards_remaining_reserve {
                    to_distribute = community_rewards_remaining_reserve;
                    community_rewards_remaining_reserve_mapper.clear();
                    self.produce_community_rewards_enabled().set(false);
                } else {
                    community_rewards_remaining_reserve_mapper
                        .update(|total| *total -= &to_distribute);
                }
            }
            self.last_reward_block_nonce().set(&current_block_nonce);
            to_distribute
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(&self, storage: &mut StorageCache<Self::Api>) {
        let total_reward = self.distribute_per_block_rewards();

        if total_reward > 0u64 {
            storage.reward_reserve += &total_reward;

            if storage.farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage.division_safety_constant)
                    / &storage.farm_token_supply;
                storage.reward_per_share += &increase;
            }
        }
    }

    #[only_admin]
    #[endpoint]
    fn end_produce_rewards(&self) {
        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share().set(&storage.reward_per_share);
        self.reward_reserve().set(&storage.reward_reserve);

        self.produce_community_rewards_enabled().set(false);
    }

    #[only_admin]
    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        // Allow 0 tokens per block distribution case
        // require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        if per_block_amount > 0u64 {
            let community_rewards_remaining_reserve =
                self.community_rewards_remaining_reserve().get();
            let actual_rewarding_blocks_no = community_rewards_remaining_reserve / per_block_amount.clone();
            require!(
                actual_rewarding_blocks_no >= MINIMUM_REWARDING_BLOCKS,
                "Not enough rewards for at least 3 months"
            );
        }

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share().set(&storage.reward_per_share);
        self.reward_reserve().set(&storage.reward_reserve);

        self.per_block_reward_amount().set(&per_block_amount);
    }
}
