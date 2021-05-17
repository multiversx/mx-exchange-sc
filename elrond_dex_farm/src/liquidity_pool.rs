elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINIMUM_INITIAL_FARM_AMOUNT: u64 = 1000;

use super::config;
use super::rewards;

#[elrond_wasm_derive::module]
pub trait LiquidityPoolModule: rewards::RewardsModule + config::ConfigModule {
    fn add_liquidity(&self, amount: &Self::BigUint) -> SCResult<Self::BigUint> {
        require!(amount > &0, "Amount needs to be greater than 0");
        let reward_token_id = self.reward_token_id().get();
        let farming_token_id = self.farming_token_id().get();
        let mut total_supply = self.total_supply().get();
        let mut virtual_reserves = self.virtual_reserves().get();

        let liquidity = self.calculate_liquidity(
            amount,
            &total_supply,
            &virtual_reserves,
            &farming_token_id,
            &reward_token_id,
        );
        require!(liquidity > 0, "Insuficient liquidity minted");

        if total_supply == 0 {
            require!(
                liquidity > MINIMUM_INITIAL_FARM_AMOUNT,
                "First farm needs to be greater than minimum amount"
            );
        }
        total_supply += &liquidity;
        self.total_supply().set(&total_supply);

        if farming_token_id != reward_token_id {
            virtual_reserves += amount;
            self.virtual_reserves().set(&virtual_reserves);
        }

        Ok(liquidity)
    }

    fn remove_liquidity(
        &self,
        liquidity: &Self::BigUint,
        enter_amount: &Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(liquidity > &0, "Amount needs to be greater than 0");
        require!(enter_amount > &0, "Amount needs to be greater than 0");

        let reward_token_id = self.reward_token_id().get();
        let farming_token_id = self.farming_token_id().get();
        let mut virtual_reserves = self.virtual_reserves().get();
        let mut total_supply = self.total_supply().get();
        require!(&total_supply > liquidity, "Not enough supply");

        let reward = self.calculate_reward_for_given_liquidity(
            &liquidity,
            &enter_amount,
            &total_supply,
            &virtual_reserves,
            &reward_token_id,
        );

        total_supply -= liquidity;
        self.total_supply().set(&total_supply);

        if farming_token_id != reward_token_id {
            require!(
                &virtual_reserves > enter_amount,
                "Virtual amount is less than enter amount"
            );
            virtual_reserves -= enter_amount;
            self.virtual_reserves().set(&virtual_reserves);
        }

        Ok(reward)
    }

    fn calculate_liquidity(
        &self,
        amount: &Self::BigUint,
        total_supply: &Self::BigUint,
        virtual_reserves: &Self::BigUint,
        farming_token_id: &TokenIdentifier,
        reward_token_id: &TokenIdentifier,
    ) -> Self::BigUint {
        let mut actual_reserves = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            reward_token_id.as_esdt_identifier(),
            0,
        );
        let reward_amount = self.calculate_reward_amount_current_block();

        if farming_token_id == reward_token_id {
            actual_reserves -= amount;
        }

        if total_supply == &0 {
            amount.clone()
        } else {
            let total_reserves = virtual_reserves + &actual_reserves + reward_amount;
            amount * &total_supply / total_reserves
        }
    }

    fn is_first_provider(&self) -> bool {
        self.total_supply().get() == 0
    }

    fn minimum_liquidity_farm_amount(&self) -> u64 {
        MINIMUM_INITIAL_FARM_AMOUNT
    }

    #[view(getTotalSupply)]
    #[storage_mapper("total_supply")]
    fn total_supply(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getVirtualReserves)]
    #[storage_mapper("virtual_reserves")]
    fn virtual_reserves(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
