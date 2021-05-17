elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINIMUM_INITIAL_FARM_AMOUNT: u64 = 1000;

use super::config;
use super::rewards;

#[elrond_wasm_derive::module]
pub trait LiquidityPoolModule: rewards::RewardsModule + config::ConfigModule {
    fn add_liquidity(&self, amount: &Self::BigUint) -> SCResult<Self::BigUint> {
        require!(amount > &0, "Amount needs to be greater than 0");
        let mut farm_token_supply = self.farm_token_supply().get();
        let mut virtual_reserves = self.virtual_reserves().get();

        let liquidity = self.calculate_liquidity(
            amount,
            &farm_token_supply,
            &virtual_reserves,
        );
        require!(liquidity > 0, "Insuficient liquidity minted");

        if farm_token_supply == 0 {
            require!(
                liquidity > MINIMUM_INITIAL_FARM_AMOUNT,
                "First farm needs to be greater than minimum amount"
            );
        }
        farm_token_supply += &liquidity;
        self.farm_token_supply().set(&farm_token_supply);

        virtual_reserves += amount;
        self.virtual_reserves().set(&virtual_reserves);

        Ok(liquidity)
    }

    fn remove_liquidity(
        &self,
        liquidity: &Self::BigUint,
        enter_amount: &Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        require!(liquidity > &0, "Amount needs to be greater than 0");
        require!(enter_amount > &0, "Amount needs to be greater than 0");

        let mut virtual_reserves = self.virtual_reserves().get();
        let mut farm_token_supply = self.farm_token_supply().get();
        let mut actual_reserves = self.actual_reserves().get();
        let reward = self.calculate_reward_for_given_liquidity(
            &liquidity,
            &enter_amount,
            &farm_token_supply,
            &virtual_reserves,
            &actual_reserves,
        );

        //These are sanity checks. Should never fail.
        require!(&farm_token_supply > liquidity, "Not enough supply");
        require!(&virtual_reserves > enter_amount, "Not enough virtual amount");
        require!(actual_reserves >= reward, "Not enough actual reserves");

        actual_reserves -= &reward;
        self.actual_reserves().set(&actual_reserves);

        farm_token_supply -= liquidity;
        self.farm_token_supply().set(&farm_token_supply);

        virtual_reserves -= enter_amount;
        self.virtual_reserves().set(&virtual_reserves);

        Ok(reward)
    }

    fn calculate_liquidity(
        &self,
        amount: &Self::BigUint,
        farm_token_supply: &Self::BigUint,
        virtual_reserves: &Self::BigUint,
    ) -> Self::BigUint {
        let actual_reserves = self.actual_reserves().get();
        let reward_amount = self.calculate_reward_amount_current_block();

        if farm_token_supply == &0 {
            amount.clone()
        } else {
            let total_reserves = virtual_reserves + &actual_reserves + reward_amount;
            amount * &farm_token_supply / total_reserves
        }
    }

    fn is_first_provider(&self) -> bool {
        self.farm_token_supply().get() == 0
    }

    fn minimum_liquidity_farm_amount(&self) -> u64 {
        MINIMUM_INITIAL_FARM_AMOUNT
    }

    fn increase_actual_reserves(&self, amount: &Self::BigUint) {
        if amount > &0 {
            let mut actual_reserves = self.actual_reserves().get();
            actual_reserves += amount;
            self.actual_reserves().set(&actual_reserves);
        }
    }

    #[view(getTotalSupply)]
    #[storage_mapper("farm_token_supply")]
    fn farm_token_supply(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getVirtualReserves)]
    #[storage_mapper("virtual_reserves")]
    fn virtual_reserves(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getActualReserves)]
    #[storage_mapper("actual_reserves")]
    fn actual_reserves(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
