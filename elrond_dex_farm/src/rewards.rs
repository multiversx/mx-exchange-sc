use elrond_wasm::only_owner;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

#[elrond_wasm_derive::module]
pub trait RewardsModule {
    #[endpoint(setPerBlockRewardAmount)]
    fn start_produce_per_block_rewards(&self, per_block_amount: u64) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.per_block_reward_amount().set(&per_block_amount);
        self.last_reward_block_nonce()
            .set(&self.blockchain().get_block_nonce());
        Ok(())
    }

    fn calculate_reward_amount_current_block(&self) -> Self::BigUint {
        let current_nonce = self.blockchain().get_block_nonce();
        self.calculate_reward_amount(current_nonce)
    }

    fn calculate_reward_amount(&self, block_nonce: Nonce) -> Self::BigUint {
        let last_reward_nonce = self.last_reward_block_nonce().get();
        let per_block_reward = self.per_block_reward_amount().get();
        if block_nonce > last_reward_nonce && per_block_reward > 0 {
            Self::BigUint::from(per_block_reward)
                * Self::BigUint::from(block_nonce - last_reward_nonce)
        } else {
            Self::BigUint::zero()
        }
    }

    fn mint_rewards(&self, token_id: &TokenIdentifier) -> Self::BigUint {
        let current_nonce = self.blockchain().get_block_nonce();
        let to_mint = self.calculate_reward_amount(current_nonce);
        if to_mint != 0 {
            self.send().esdt_local_mint(
                self.blockchain().get_gas_left(),
                token_id.as_esdt_identifier(),
                &to_mint,
            );
            self.last_reward_block_nonce().set(&current_nonce);
        }
        to_mint
    }

    fn calculate_reward_for_given_liquidity(
        &self,
        liquidity: &Self::BigUint,
        enter_amount: &Self::BigUint,
        farm_token_supply: &Self::BigUint,
        virtual_reserves: &Self::BigUint,
        actual_reserves: &Self::BigUint,
    ) -> Self::BigUint {
        let big_zero = Self::BigUint::zero();
        let reward_amount = self.calculate_reward_amount_current_block();
        let total_reserves = virtual_reserves + &actual_reserves + reward_amount;

        let worth = if farm_token_supply > &0 {
            liquidity * &total_reserves / farm_token_supply.clone()
        } else {
            big_zero.clone()
        };

        if &worth > enter_amount {
            &worth - enter_amount
        } else {
            big_zero
        }
    }

    #[view(getLastRewardEpoch)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<Self::Storage, u64>;
}
