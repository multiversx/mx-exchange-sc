#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

#[elrond_wasm::module]
pub trait RewardsModule:
    config::ConfigModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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

    fn mint_per_block_rewards<MintFunction: Fn(&TokenIdentifier, &BigUint)>(
        &self,
        token_id: &TokenIdentifier,
        mint_function: MintFunction,
    ) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();
        if current_block_nonce > last_reward_nonce {
            let to_mint = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
            if to_mint != 0 {
                mint_function(token_id, &to_mint);
            }

            self.last_reward_block_nonce().set(current_block_nonce);

            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn calculate_reward(
        &self,
        farm_token_amount: &BigUint,
        farm_token_reward_per_share: &BigUint,
        current_reward_per_share: &BigUint,
        div_safety_const: &BigUint,
    ) -> BigUint {
        if current_reward_per_share > farm_token_reward_per_share {
            let rps_diff = current_reward_per_share - farm_token_reward_per_share;
            farm_token_amount * &rps_diff / div_safety_const
        } else {
            BigUint::zero()
        }
    }

    // Move to the main file
    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_is_admin();
        self.start_produce_rewards();
    }

    fn start_produce_rewards(&self) {
        require!(
            self.per_block_reward_amount().get() != 0u64,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );
        let current_nonce = self.blockchain().get_block_nonce();
        self.produce_rewards_enabled().set(true);
        self.last_reward_block_nonce().set(current_nonce);
    }

    #[inline]
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
