#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;
use common_errors::ERROR_BAD_INPUT_TOKEN;

pub const MINIMUM_REWARDING_BLOCKS: u64 = 1_296_000; // 3 months

#[elrond_wasm::module]
pub trait CommunityRewardsModule:
    config::ConfigModule
    + farm_token::FarmTokenModule
    + token_send::TokenSendModule
    + pausable::PausableModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[only_owner]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self) {
        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        require!(
            payment_token == self.reward_token_id().get(),
            ERROR_BAD_INPUT_TOKEN
        );

        self.community_rewards_remaining_reserve()
            .update(|total| *total += &payment_amount);
    }

    #[only_owner]
    #[endpoint(startProduceCommunityRewards)]
    fn start_produce_community_rewards(
        &self,
        starting_block_offset: Nonce
    ) {
        require!(
            self.community_rewards_remaining_reserve().get() != 0u64,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_community_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );

        let community_rewards_remaining_reserve = self.community_rewards_remaining_reserve().get();
        let per_block_reward_amount = self.per_block_reward_amount().get();
        let actual_rewarding_blocks_no =
            community_rewards_remaining_reserve / per_block_reward_amount;
        require!(
            actual_rewarding_blocks_no >= MINIMUM_REWARDING_BLOCKS,
            "Not enough rewards for at least 3 months"
        );

        let current_block = self.blockchain().get_block_nonce();
        let starting_block = current_block + starting_block_offset;

        self.last_reward_block_nonce().set(starting_block);
        self.produce_community_rewards_enabled().set(true);
    }

    fn calculate_per_block_community_rewards(
        &self,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint {
        if current_block_nonce <= last_reward_block_nonce || !self.produces_per_block_community_rewards() {
            return BigUint::zero();
        }

        let per_block_reward = self.per_block_reward_amount().get();
        let block_nonce_diff = current_block_nonce - last_reward_block_nonce;

        per_block_reward * block_nonce_diff
    }

    #[inline(always)]
    fn produces_per_block_community_rewards(&self) -> bool {
        self.produce_community_rewards_enabled().get()
    }

    #[storage_mapper("produce_community_rewards_enabled")]
    fn produce_community_rewards_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getCommunityRewardsRemainingReserve)]
    #[storage_mapper("community_rewards_remaining_reserve")]
    fn community_rewards_remaining_reserve(&self) -> SingleValueMapper<BigUint>;
}
