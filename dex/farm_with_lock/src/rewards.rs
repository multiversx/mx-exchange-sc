elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::farm_token;

use super::config;

use common_structs::Nonce;

#[elrond_wasm::module]
pub trait RewardsModule:
    config::ConfigModule + token_send::TokenSendModule + farm_token::FarmTokenModule
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

    fn mint_per_block_rewards(&self, _token_id: &TokenIdentifier) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_block_nonce > last_reward_nonce {
            let to_mint = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

            //Skip the actual minting. Since this SC will deliver locked rewards.
            // if to_mint != 0 {
            //     self.send().esdt_local_mint(token_id, 0, &to_mint);
            // }
            self.last_reward_block_nonce().set(&current_block_nonce);
            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(&self, reward_token_id: &TokenIdentifier) {
        let total_reward = self.mint_per_block_rewards(reward_token_id);

        if total_reward > 0 {
            self.increase_reward_reserve(&total_reward);
            self.update_reward_per_share(&total_reward);
        }
    }

    fn increase_reward_reserve(&self, amount: &BigUint) {
        self.reward_reserve().update(|reserve| {
            *reserve += amount;
        });
    }

    fn decrease_reward_reserve(&self, amount: &BigUint) -> SCResult<()> {
        self.reward_reserve().update(|reserve| {
            require!(&*reserve >= amount, "Not enough reserves");
            *reserve -= amount;
            Ok(())
        })
    }

    fn update_reward_per_share(&self, reward_increase: &BigUint) {
        let current = self.reward_per_share().get();
        let farm_token_supply = self.get_farm_token_supply();

        if farm_token_supply > 0 {
            let increase = self.calculate_reward_per_share_increase(reward_increase);

            if increase > 0 {
                self.reward_per_share().set(&(current + increase));
            }
        }
    }

    fn calculate_reward_per_share_increase(&self, reward_increase: &BigUint) -> BigUint {
        reward_increase * &self.division_safety_constant().get() / self.get_farm_token_supply()
    }

    fn calculate_reward(
        &self,
        amount: &BigUint,
        current_reward_per_share: &BigUint,
        initial_reward_per_share: &BigUint,
    ) -> BigUint {
        if current_reward_per_share > initial_reward_per_share {
            let reward_per_share_diff = current_reward_per_share - initial_reward_per_share;
            amount * &reward_per_share_diff / self.division_safety_constant().get()
        } else {
            BigUint::zero()
        }
    }

    #[endpoint]
    fn start_produce_rewards(&self) -> SCResult<()> {
        self.require_permissions()?;
        require!(
            self.per_block_reward_amount().get() != 0,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );
        let current_nonce = self.blockchain().get_block_nonce();
        self.produce_rewards_enabled().set(&true);
        self.last_reward_block_nonce().set(&current_nonce);
        Ok(())
    }

    #[endpoint]
    fn end_produce_rewards(&self) -> SCResult<()> {
        self.require_permissions()?;
        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);
        self.produce_rewards_enabled().set(&false);
        Ok(())
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) -> SCResult<()> {
        self.require_permissions()?;
        require!(per_block_amount != 0, "Amount cannot be zero");
        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);
        self.per_block_reward_amount().set(&per_block_amount);
        Ok(())
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
