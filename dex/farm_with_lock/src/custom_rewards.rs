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
            *storage.reward_reserve.as_mut().unwrap() += &total_reward;

            if storage.farm_token_supply.as_ref().unwrap() != &0u64 {
                let increase = total_reward * storage.division_safety_constant.as_ref().unwrap()
                    / storage.farm_token_supply.as_ref().unwrap();
                *storage.reward_per_share.as_mut().unwrap() += &increase;
            }
        }
    }

    #[endpoint]
    fn end_produce_rewards(&self) {
        self.require_permissions();

        // TODO: duplicated code
        let mut storage = StorageCache {
            reward_token_id: Some(self.reward_token_id().get()),
            division_safety_constant: Some(self.division_safety_constant().get()),
            farm_token_supply: Some(self.farm_token_supply().get()),
            reward_reserve: Some(self.reward_reserve().get()),
            reward_per_share: Some(self.reward_per_share().get()),
            ..Default::default()
        };

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share()
            .set(storage.reward_per_share.as_ref().unwrap());
        self.reward_reserve()
            .set(storage.reward_reserve.as_ref().unwrap());

        self.produce_rewards_enabled().set(&false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_permissions();
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        // TODO: duplicated code
        let mut storage = StorageCache {
            reward_token_id: Some(self.reward_token_id().get()),
            division_safety_constant: Some(self.division_safety_constant().get()),
            farm_token_supply: Some(self.farm_token_supply().get()),
            reward_reserve: Some(self.reward_reserve().get()),
            reward_per_share: Some(self.reward_per_share().get()),
            ..Default::default()
        };

        self.generate_aggregated_rewards(&mut storage);
        self.reward_per_share()
            .set(storage.reward_per_share.as_ref().unwrap());
        self.reward_reserve()
            .set(storage.reward_reserve.as_ref().unwrap());

        self.per_block_reward_amount().set(&per_block_amount);
    }
}
