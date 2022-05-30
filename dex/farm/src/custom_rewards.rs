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
    fn mint_per_block_rewards(&self, token_id: &TokenIdentifier) -> BigUint {
        let current_checkpoint_block_nonce = self.current_checkpoint_block_nonce().get();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_checkpoint_block_nonce > last_reward_nonce {
            let to_mint =
                self.calculate_per_block_rewards(current_checkpoint_block_nonce, last_reward_nonce);

            if to_mint != 0 {
                self.send().esdt_local_mint(token_id, 0, &to_mint);
            }
            self.last_reward_block_nonce()
                .set(&current_checkpoint_block_nonce);
            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(&self, storage: &mut StorageCache<Self::Api>) {
        let total_reward = self.mint_per_block_rewards(&storage.reward_token_id);

        if total_reward > 0u64 {
            storage.reward_reserve += &total_reward;

            if storage.global_farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage.division_safety_constant)
                    / &storage.global_farm_token_supply;
                storage.reward_per_share += &increase;
            }
        }
    }

    #[only_owner]
    #[endpoint]
    fn end_produce_rewards(&self) {
        self.produce_rewards_enabled().set(false);
    }

    #[only_owner]
    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        self.per_block_reward_amount().set(&per_block_amount);
    }
}
