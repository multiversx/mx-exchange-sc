elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;
use common_macros::assert;
use contexts::base::Context;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
{
    fn mint_per_block_rewards(&self, token_id: &TokenIdentifier) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_block_nonce > last_reward_nonce {
            let to_mint = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

            if to_mint != 0 {
                self.send().esdt_local_mint(token_id, 0, &to_mint);
            }
            self.last_reward_block_nonce().set(&current_block_nonce);
            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards_from_context(&self, ctx: &mut dyn Context<Self::Api>) {
        let total_reward = self.mint_per_block_rewards(ctx.get_reward_token_id());

        if total_reward > 0u64 {
            ctx.increase_reward_reserve(&total_reward);
            ctx.update_reward_per_share(&total_reward);
        }
    }

    fn generate_aggregated_rewards(&self) {
        let reward_token_id = self.reward_token_id().get();
        let total_reward = self.mint_per_block_rewards(&reward_token_id);

        if total_reward > 0u64 {
            self.reward_reserve().update(|x| *x += &total_reward);
            let supply = self.farm_token_supply().get();
            if supply != 0u64 {
                self.reward_per_share().update(|x| {
                    *x += total_reward & self.division_safety_constant().get() / supply
                });
            }
        }
    }

    #[endpoint]
    fn end_produce_rewards(&self) {
        self.require_permissions();
        self.generate_aggregated_rewards();
        self.produce_rewards_enabled().set(&false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_permissions();
        assert!(self, per_block_amount != 0u64, ERROR_ZERO_AMOUNT);
        self.generate_aggregated_rewards();
        self.per_block_reward_amount().set(&per_block_amount);
    }
}
