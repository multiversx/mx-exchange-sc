elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINIMUM_INITIAL_FARM_AMOUNT: u64 = 1000;

use super::rewards;

#[elrond_wasm_derive::module]
pub trait LiquidityPoolModule: rewards::RewardsModule {
    fn add_liquidity(
        &self,
        amount: Self::BigUint,
        farming_pool_token_id: TokenIdentifier,
        farmed_token_id: TokenIdentifier,
    ) -> SCResult<Self::BigUint> {
        require!(amount > 0, "Amount needs to be greater than 0");

        let liquidity = self.calculate_liquidity(&amount, &farming_pool_token_id, &farmed_token_id);
        require!(liquidity > 0, "Insuficient liquidity minted");

        let mut total_supply = self.total_supply().get();
        if total_supply == 0 {
            require!(
                liquidity > MINIMUM_INITIAL_FARM_AMOUNT,
                "First farm needs to be greater than minimum amount"
            );
        }

        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        if is_virtual_amount {
            let mut virtual_reserves = self.virtual_reserves().get();
            virtual_reserves += amount;
            self.virtual_reserves().set(&virtual_reserves);
        }

        total_supply += &liquidity;
        self.total_supply().set(&total_supply);

        Ok(liquidity)
    }

    fn remove_liquidity(
        &self,
        liquidity: Self::BigUint,
        initial_worth: Self::BigUint,
        farming_pool_token_id: TokenIdentifier,
        farmed_token_id: TokenIdentifier,
    ) -> SCResult<Self::BigUint> {
        let mut total_supply = self.total_supply().get();
        let mut virtual_reserves = self.virtual_reserves().get();

        let reward = self.calculate_reward_for_given_liquidity(
            &liquidity,
            &initial_worth,
            &farming_pool_token_id,
            &total_supply,
            &virtual_reserves,
        )?;

        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        if is_virtual_amount {
            require!(
                virtual_reserves > initial_worth,
                "Removing more virtual amount than available"
            );
            virtual_reserves -= initial_worth;
            self.virtual_reserves().set(&virtual_reserves);
        }

        total_supply -= liquidity;
        self.total_supply().set(&total_supply);

        Ok(reward)
    }

    fn calculate_liquidity(
        &self,
        amount: &Self::BigUint,
        farming_pool_token_id: &TokenIdentifier,
        farmed_token_id: &TokenIdentifier,
    ) -> Self::BigUint {
        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        let total_supply = self.total_supply().get();
        let virtual_reserves = self.virtual_reserves().get();
        let mut actual_reserves = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            farming_pool_token_id,
            0,
        );
        let reward_amount = self.calculate_reward_amount_current_block();

        if !is_virtual_amount {
            actual_reserves -= amount;
        }

        if total_supply == 0 {
            amount.clone()
        } else {
            let total_reserves = virtual_reserves + actual_reserves + reward_amount;
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
