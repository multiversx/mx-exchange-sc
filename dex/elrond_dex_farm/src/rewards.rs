elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use super::config;

type Nonce = u64;

#[elrond_wasm_derive::module]
pub trait RewardsModule: config::ConfigModule {
    fn calculate_per_block_rewards(&self, block_nonce: Nonce) -> Self::BigUint {
        let big_zero = Self::BigUint::zero();

        if self.produces_per_block_rewards() {
            let last_reward_nonce = self.last_reward_block_nonce().get();
            let per_block_reward = self.per_block_reward_amount().get();

            if block_nonce > last_reward_nonce {
                per_block_reward * Self::BigUint::from(block_nonce - last_reward_nonce)
            } else {
                big_zero
            }
        } else {
            big_zero
        }
    }

    fn mint_per_block_rewards(&self, token_id: &TokenIdentifier) -> Self::BigUint {
        let current_nonce = self.blockchain().get_block_nonce();
        let to_mint = self.calculate_per_block_rewards(current_nonce);

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

    fn generate_aggregated_rewards(&self, reward_token_id: &TokenIdentifier) {
        let reward_minted = self.mint_per_block_rewards(&reward_token_id);
        self.increase_current_block_fee_storage(&Self::BigUint::zero());
        let fees = self.undistributed_fee_storage().get();
        self.undistributed_fee_storage().clear();
        let total_reward = reward_minted + fees;

        if total_reward > 0 {
            self.increase_reward_reserve(&total_reward);
            self.update_reward_per_share(&total_reward);
        }
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
            let increase = self.calculate_reward_per_share_increase(reward_increase);

            if increase > 0 {
                self.reward_per_share().set(&(current + increase));
            }
        }
    }

    fn calculate_reward_per_share_increase(
        &self,
        reward_increase: &Self::BigUint,
    ) -> Self::BigUint {
        reward_increase * &self.division_safety_constant().get() / self.farm_token_supply().get()
    }

    fn calculate_reward(
        &self,
        amount: &Self::BigUint,
        current_reward_per_share: &Self::BigUint,
        initial_reward_per_share: &Self::BigUint,
    ) -> Self::BigUint {
        let reward_per_share_diff = current_reward_per_share - initial_reward_per_share;
        amount * &reward_per_share_diff / self.division_safety_constant().get()
    }

    fn increase_undistributed_fee_storage(&self, amount: &Self::BigUint) {
        if amount > &0 {
            let current = self.undistributed_fee_storage().get();
            self.undistributed_fee_storage().set(&(&current + amount));
        }
    }

    fn increase_current_block_fee_storage(&self, amount: &Self::BigUint) {
        let current_block = self.blockchain().get_block_nonce();
        let current_block_fee_storage = self.current_block_fee_storage().get();

        let (known_block_nonce, fee_amount) = match current_block_fee_storage {
            Some(value) => (value.0, value.1),
            None => (0, Self::BigUint::zero()),
        };

        if known_block_nonce == current_block {
            if amount > &0 {
                self.current_block_fee_storage()
                    .set(&Some((current_block, &fee_amount + amount)));
            }
        } else {
            self.increase_undistributed_fee_storage(&fee_amount);
            self.current_block_fee_storage()
                .set(&Some((current_block, amount.clone())));
        }
    }

    #[endpoint]
    fn start_produce_rewards(&self) -> SCResult<()> {
        self.require_permissions()?;
        require!(
            self.per_block_reward_amount().get() != 0,
            "Cannot produce zero reward amount"
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
        self.last_reward_block_nonce().set(&0);
        Ok(())
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: Self::BigUint) -> SCResult<()> {
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
    fn reward_per_share(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[storage_mapper("undistributed_fee_storage")]
    fn undistributed_fee_storage(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[storage_mapper("current_block_fee_storage")]
    fn current_block_fee_storage(
        &self,
    ) -> SingleValueMapper<Self::Storage, Option<(Nonce, Self::BigUint)>>;
}
