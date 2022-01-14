elrond_wasm::imports!();
elrond_wasm::derive_imports!();

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

    fn generate_aggregated_rewards<MintRewardsFunc: Fn(&Self, &TokenIdentifier) -> BigUint>(
        &self,
        reward_token_id: &TokenIdentifier,
        mint_rewards_function: MintRewardsFunc,
    ) {
        let total_reward = mint_rewards_function(self, reward_token_id);
        if total_reward > 0 {
            self.increase_reward_reserve(&total_reward);
            self.update_reward_per_share(&total_reward);
        }
    }
}
