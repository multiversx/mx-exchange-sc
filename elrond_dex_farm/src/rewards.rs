elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use super::config;

type Nonce = u64;
const DIVISION_SAFETY_CONSTANT: u64 = 1000000000000;

#[elrond_wasm_derive::module]
pub trait RewardsModule: config::ConfigModule {
    fn calculate_blocks_reward(&self, block_nonce: Nonce) -> Self::BigUint {
        let big_zero = Self::BigUint::zero();

        if self.produces_rewards() {
            let last_reward_nonce = self.last_reward_block_nonce().get();
            let per_block_reward = self.per_block_reward_amount().get();

            if block_nonce > last_reward_nonce && per_block_reward > 0 {
                Self::BigUint::from(per_block_reward)
                    * Self::BigUint::from(block_nonce - last_reward_nonce)
            } else {
                big_zero
            }
        } else {
            big_zero
        }
    }

    fn mint_rewards(&self, token_id: &TokenIdentifier) -> Self::BigUint {
        let current_nonce = self.blockchain().get_block_nonce();
        let to_mint = self.calculate_blocks_reward(current_nonce);
        if to_mint != 0 {
            self.send().esdt_local_mint(
                self.mint_tokens_gas_limit().get(),
                token_id.as_esdt_identifier(),
                &to_mint,
            );
            self.last_reward_block_nonce().set(&current_nonce);
        }
        to_mint
    }

    fn generate_rewards(&self, reward_token_id: &TokenIdentifier) {
        let reward_minted = self.mint_rewards(&reward_token_id);
        if reward_minted > 0 {
            self.increase_reward_reserve(&reward_minted);
            self.update_reward_per_share(&reward_minted);
        }
    }

    fn calculate_reward(
        &self,
        amount: &Self::BigUint,
        enter_reward_per_share: &Self::BigUint,
    ) -> Self::BigUint {
        amount * &(&self.reward_per_share().get() - enter_reward_per_share) / Self::BigUint::from(DIVISION_SAFETY_CONSTANT)
    }

    fn increase_reward_reserve(&self, amount: &Self::BigUint) {
        let current = self.reward_reserve().get();
        self.reward_reserve().set(&(&current + amount));
    }

    fn decrease_reward_reserve(&self, amount: &Self::BigUint) -> SCResult<()> {
        let current = self.reward_reserve().get();
        require!(&current >= amount, "Not enough reserves");
        self.reward_reserve().set(&(&current - amount));
        Ok(())
    }

    fn update_reward_per_share(&self, reward_increase: &Self::BigUint) {
        let current = self.reward_per_share().get();
        let farm_token_supply = self.farm_token_supply().get();
        if farm_token_supply > 0 {
            let increase = reward_increase * &Self::BigUint::from(DIVISION_SAFETY_CONSTANT)
                / self.farm_token_supply().get();
            if increase > 0 {
                self.reward_per_share().set(&(current + increase));
            }
        }
    }

    #[view(getRewardPerShare)]
    #[storage_mapper("reward_per_share")]
    fn reward_per_share(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
