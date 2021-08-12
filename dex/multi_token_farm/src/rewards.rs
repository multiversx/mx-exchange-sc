elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

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
            Self::BigUint::from(per_block_reward) * (block_nonce - last_reward_nonce).into()
        } else {
            0u64.into()
        }
    }

    fn mint_rewards(&self, token_id: &TokenIdentifier) {
        let current_nonce = self.blockchain().get_block_nonce();
        let to_mint = self.calculate_reward_amount(current_nonce);
        if to_mint != 0 {
            self.send().esdt_local_mint(token_id, 0, &to_mint);
            self.last_reward_block_nonce().set(&current_nonce);
        }
    }

    fn calculate_reward_for_given_liquidity(
        &self,
        liquidity: &Self::BigUint,
        initial_worth: &Self::BigUint,
        token_id: &TokenIdentifier,
        total_supply: &Self::BigUint,
        virtual_reserves: &Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(liquidity > &0, "Liquidity needs to be greater than 0");
        require!(
            total_supply > liquidity,
            "Removing more liquidity than existent"
        );

        let actual_reserves =
            self.blockchain()
                .get_esdt_balance(&self.blockchain().get_sc_address(), token_id, 0);
        let reward_amount = self.calculate_reward_amount_current_block();

        let total_reserves = virtual_reserves + &actual_reserves + reward_amount;
        let worth = liquidity * &total_reserves / total_supply.clone();

        let reward = if &worth > initial_worth {
            &worth - initial_worth
        } else {
            0u64.into()
        };

        Ok(reward)
    }

    #[view(getLastRewardEpoch)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<Self::Storage, u64>;
}
