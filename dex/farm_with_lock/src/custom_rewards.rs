elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;

#[elrond_wasm::module]
pub trait CustomRewardsModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn mint_per_block_rewards(&self) -> BigUint {
        let current_checkpoint_block_nonce = self.current_checkpoint_block_nonce().get();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_checkpoint_block_nonce > last_reward_nonce {
            let local_farm_token_supply = self.local_farm_token_supply().get();
            let global_farm_token_supply = self.global_farm_token_supply().get();

            let to_mint = self.calculate_per_block_rewards(
                current_checkpoint_block_nonce,
                last_reward_nonce,
                &local_farm_token_supply,
                &global_farm_token_supply,
            );

            // Skip the actual minting. Since this SC will deliver locked rewards.

            self.last_reward_block_nonce()
                .set(&current_checkpoint_block_nonce);
            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(&self) {
        let total_reward = self.mint_per_block_rewards();

        if total_reward > 0u64 {
            self.reward_reserve()
                .update(|reward_reserve| *reward_reserve += &total_reward);

            let local_farm_token_supply = self.local_farm_token_supply().get();
            if local_farm_token_supply != 0u64 {
                let division_safety_constant = self.division_safety_constant().get();
                let increase = total_reward * &division_safety_constant / &local_farm_token_supply;
                self.reward_per_share()
                    .update(|reward_per_share| *reward_per_share += &increase);
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
