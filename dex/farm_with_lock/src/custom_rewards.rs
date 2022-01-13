elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::custom_config;
use crate::contexts::base::Context;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
    + custom_config::CustomConfigModule
{
    fn mint_per_block_rewards(&self, ctx: &mut dyn Context<Self::Api>) -> BigUint {
        let current_block_nonce = ctx.get_block_nonce();
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

    fn generate_aggregated_rewards(&self, ctx: &mut dyn Context<Self::Api>) {
        let total_reward = self.mint_per_block_rewards(ctx);

        if total_reward > 0u64 {
            ctx.increase_reward_reserve(&total_reward);
            ctx.update_reward_per_share(&total_reward);
        }
    }

    #[endpoint]
    fn end_produce_rewards(&self) {
        // self.require_permissions()?;
        // self.generate_aggregated_rewards();
        // self.produce_rewards_enabled().set(&false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        // self.require_permissions()?;
        // assert!(self, per_block_amount != 0u64, ERROR_ZERO_AMOUNT);
        // self.generate_aggregated_rewards();
        // self.per_block_reward_amount().set(&per_block_amount);
    }
}
