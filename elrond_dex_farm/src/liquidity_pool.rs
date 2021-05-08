elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINIMUM_INITIAL_FARM_AMOUNT: u64 = 1000;

pub use crate::rewards::*;

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {
    #[module(RewardsModule)]
    fn rewards(&self) -> RewardsModule<T, BigInt, BigUint>;

    fn add_liquidity(
        &self,
        amount: BigUint,
        farming_pool_token_id: TokenIdentifier,
        farmed_token_id: TokenIdentifier,
    ) -> SCResult<BigUint> {
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

        total_supply += liquidity.clone();
        self.total_supply().set(&total_supply);

        Ok(liquidity)
    }

    fn remove_liquidity(
        &self,
        liquidity: BigUint,
        initial_worth: BigUint,
        farming_pool_token_id: TokenIdentifier,
        farmed_token_id: TokenIdentifier,
    ) -> SCResult<BigUint> {
        let reward = sc_try!(self.rewards().calculate_reward_for_given_liquidity(
            liquidity.clone(),
            initial_worth.clone(),
            farming_pool_token_id.clone()
        ));

        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        if is_virtual_amount {
            let mut virtual_reserves = self.virtual_reserves().get();
            require!(
                virtual_reserves > initial_worth,
                "Removing more virtual amount than available"
            );
            virtual_reserves -= initial_worth;
            self.virtual_reserves().set(&virtual_reserves);
        }

        let mut total_supply = self.total_supply().get();
        total_supply -= liquidity;
        self.total_supply().set(&total_supply);

        Ok(reward)
    }

    fn calculate_liquidity(
        &self,
        amount: &BigUint,
        farming_pool_token_id: &TokenIdentifier,
        farmed_token_id: &TokenIdentifier,
    ) -> BigUint {
        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        let total_supply = self.total_supply().get();
        let virtual_reserves = self.virtual_reserves().get();
        let mut actual_reserves = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            farming_pool_token_id.as_esdt_identifier(),
            0,
        );
        let reward_amount = self.rewards().calculate_reward_amount_current_block();

        if !is_virtual_amount {
            actual_reserves -= amount.clone();
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
    fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;

    #[view(getVirtualReserves)]
    #[storage_mapper("virtual_reserves")]
    fn virtual_reserves(&self) -> SingleValueMapper<Self::Storage, BigUint>;
}
