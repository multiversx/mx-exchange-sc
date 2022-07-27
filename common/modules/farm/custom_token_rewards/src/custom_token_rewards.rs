#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

use common_errors::{ERROR_BAD_INPUT_TOKEN, ERROR_NOT_AN_ESDT};

#[elrond_wasm::module]
pub trait CustomTokenRewardsModule:
    config::ConfigModule
    + farm_token::FarmTokenModule
    + token_send::TokenSendModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn calculate_per_block_custom_rewards(
        &self,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint {
        let final_reward_block = self.custom_token_final_reward_block().get();
        if current_block_nonce <= last_reward_block_nonce
            || final_reward_block < current_block_nonce
            || !self.produces_per_block_custom_rewards()
        {
            return BigUint::zero();
        }

        let mut rewards = BigUint::zero();
        if final_reward_block == current_block_nonce {
            let remaining_custom_rewards = self.remaining_custom_rewards_reserve();
            rewards = remaining_custom_rewards.get();
            remaining_custom_rewards.set(BigUint::zero());
        } else {
            let per_block_reward = self.custom_rewards_per_block().get();
            let block_nonce_diff = current_block_nonce - last_reward_block_nonce;
            let rewards = per_block_reward * block_nonce_diff;

            self.remaining_custom_rewards_reserve()
                .update(|total| *total -= &rewards);
        }

        rewards
    }

    #[endpoint(setCustomRewardToken)]
    fn set_custom_reward_token(&self, token_id: TokenIdentifier) {
        self.require_caller_is_admin();
        require!(token_id.is_valid_esdt_identifier(), ERROR_NOT_AN_ESDT);

        self.custom_reward_token().set(token_id);
    }

    #[payable("*")]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self, final_reward_block: Nonce) {
        self.require_caller_is_admin();
        self.start_produce_custom_rewards();

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();

        require!(
            payment_token == self.custom_reward_token().get(),
            ERROR_BAD_INPUT_TOKEN
        );

        self.custom_token_final_reward_block()
            .set(final_reward_block);
        self.remaining_custom_rewards_reserve()
            .update(|total| *total += &payment_amount);
        self.calculate_per_block_custom_rewards_rate();
    }

    #[endpoint(startProduceCustomRewards)]
    fn start_produce_custom_rewards(&self) {
        require!(
            self.remaining_custom_rewards_reserve().get() != 0u64,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );
        self.produce_rewards_enabled().set(true);
        self.calculate_per_block_custom_rewards_rate();
    }

    #[endpoint(pauseProduceCustomRewards)]
    fn pause_produce_custom_rewards(&self) {
        require!(
            self.produce_rewards_enabled().get(),
            "Producing rewards is already disabled"
        );

        self.produce_rewards_enabled().set(false);
    }

    fn calculate_per_block_custom_rewards_rate(&self) {
        let current_nonce = self.blockchain().get_block_nonce();
        self.last_reward_block_nonce().set(current_nonce);
        let final_reward_block = self.custom_token_final_reward_block().get();

        require!(
            final_reward_block > current_nonce,
            "Final reward block is in the past"
        );

        let remaining_custom_rewards_reserve = self.remaining_custom_rewards_reserve().get();
        let blocks_diff = final_reward_block - current_nonce;
        let custom_rewards_per_block = remaining_custom_rewards_reserve / blocks_diff;

        self.custom_rewards_per_block()
            .set(custom_rewards_per_block);
    }

    #[inline]
    fn produces_per_block_custom_rewards(&self) -> bool {
        self.produce_rewards_enabled().get()
    }

    #[view(getRemainingCustomRewardsReserve)]
    #[storage_mapper("remaining_custom_rewards_reserve")]
    fn remaining_custom_rewards_reserve(&self) -> SingleValueMapper<BigUint>;

    #[view(getCustomRewardsPerBlock)]
    #[storage_mapper("custom_rewards_per_block")]
    fn custom_rewards_per_block(&self) -> SingleValueMapper<BigUint>;

    #[view(getCustomTokenFinalRewardBlock)]
    #[storage_mapper("custom_token_final_reward_block")]
    fn custom_token_final_reward_block(&self) -> SingleValueMapper<Nonce>;

    #[view(getCustomRewardToken)]
    #[storage_mapper("custom_reward_token")]
    fn custom_reward_token(&self) -> SingleValueMapper<TokenIdentifier>;
}
