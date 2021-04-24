elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MINIMUM_INITIAL_FARM_AMOUNT: u64 = 1000;

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {
    fn add_liquidity(
        &self,
        amount: BigUint,
        farming_pool_token_id: TokenIdentifier,
        farmed_token_id: TokenIdentifier,
    ) -> SCResult<BigUint> {
        require!(amount > 0, "Amount needs to be greater than 0");

        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        let mut total_supply = self.total_supply().get();
        let mut virtual_reserves = self.virtual_reserves().get();
        let mut actual_reserves = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            farming_pool_token_id.as_esdt_identifier(),
            0,
        );
        if !is_virtual_amount {
            actual_reserves -= amount.clone();
        }

        let liquidity: BigUint;
        if total_supply == 0 {
            let minimum_amount = BigUint::from(MINIMUM_INITIAL_FARM_AMOUNT);
            require!(
                amount > minimum_amount,
                "First farm needs to be greater than minimum amount"
            );
            liquidity = amount.clone() - minimum_amount.clone();
            total_supply = minimum_amount;
            self.total_supply().set(&total_supply);
        } else {
            let total_reserves = virtual_reserves.clone() + actual_reserves;
            liquidity = amount.clone() * total_supply.clone() / total_reserves;
        }
        require!(liquidity > 0, "Insuficient liquidity minted");

        if is_virtual_amount {
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
        let reward = sc_try!(self.calculate_reward(
            liquidity.clone(),
            initial_worth.clone(),
            farming_pool_token_id.clone()
        ));

        let is_virtual_amount = farming_pool_token_id != farmed_token_id;
        if is_virtual_amount {
            let mut virtual_reserves = self.virtual_reserves().get();
            virtual_reserves -= initial_worth;
            self.virtual_reserves().set(&virtual_reserves);
        }

        let mut total_supply = self.total_supply().get();
        total_supply -= liquidity;
        self.total_supply().set(&total_supply);

        Ok(reward)
    }

    fn calculate_reward(
        &self,
        liquidity: BigUint,
        initial_worth: BigUint,
        token_id: TokenIdentifier,
    ) -> SCResult<BigUint> {
        require!(liquidity > 0, "Liquidity needs to be greater than 0");

        let total_supply = self.total_supply().get();
        require!(
            total_supply > liquidity,
            "Removing more liquidity than existent"
        );

        let virtual_reserves = self.virtual_reserves().get();
        require!(
            virtual_reserves > initial_worth,
            "Removing more virtual reserve than existent"
        );

        let actual_reserves = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            token_id.as_esdt_identifier(),
            0,
        );

        let total_reserves = virtual_reserves + actual_reserves;
        let worth = liquidity * total_reserves / total_supply;

        let reward = if worth > initial_worth {
            worth - initial_worth
        } else {
            BigUint::zero()
        };

        Ok(reward)
    }

    #[view(getTotalSupply)]
    #[storage_mapper("total_supply")]
    fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;

    #[view(getVirtualReserves)]
    #[storage_mapper("virtual_reserves")]
    fn virtual_reserves(&self) -> SingleValueMapper<Self::Storage, BigUint>;
}
