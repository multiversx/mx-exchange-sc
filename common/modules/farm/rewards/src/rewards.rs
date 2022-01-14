#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_macros::assert;
use common_structs::Nonce;

#[elrond_wasm::module]
pub trait RewardsModule:
    config::ConfigModule + farm_token::FarmTokenModule + token_send::TokenSendModule
{
    fn calculate_per_block_rewards(
        &self,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint {
        if current_block_nonce <= last_reward_block_nonce || !self.produces_per_block_rewards() {
            return BigUint::zero();
        }

        let per_block_reward = self.per_block_reward_amount().get();
        let block_nonce_diff = current_block_nonce - last_reward_block_nonce;

        per_block_reward * block_nonce_diff
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards(&self) {
        self.require_permissions();
        assert!(
            self,
            self.per_block_reward_amount().get() != 0u64,
            b"Cannot produce zero reward amount"
        );
        assert!(
            self,
            !self.produce_rewards_enabled().get(),
            b"Producing rewards is already enabled"
        );
        let current_nonce = self.blockchain().get_block_nonce();
        self.produce_rewards_enabled().set(&true);
        self.last_reward_block_nonce().set(&current_nonce);
    }

    #[inline(always)]
    fn produces_per_block_rewards(&self) -> bool {
        self.produce_rewards_enabled().get()
    }

    #[view(getRewardPerShare)]
    #[storage_mapper("reward_per_share")]
    fn reward_per_share(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<BigUint>;
}
