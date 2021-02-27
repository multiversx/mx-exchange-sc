imports!();
derive_imports!();

// pub mod liquidity_supply;

pub use crate::liquidity_supply::*;


#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {

    #[module(LiquiditySupplyModuleImpl)]
    fn supply(&self) -> LiquiditySupplyModuleImpl<T, BigInt, BigUint>;

    fn add_liquidity(
        &self,
        desired_amount_a: BigUint,
        desired_amount_b: BigUint,
    ) -> SCResult<()> {
        let caller = self.get_caller();
        let token_a = self.get_token_a_name();
		let token_b = self.get_token_b_name();
        let total_supply = self.supply().get_total_supply();
		let mut reserve_a = self.get_pair_reserve(&token_a);
		let mut reserve_b = self.get_pair_reserve(&token_b);
		let liquidity: BigUint;
        let (amount_a, amount_b) = self._add_liquidity(desired_amount_a, desired_amount_b);
        
        if total_supply == 0 {
			liquidity = amount_a.clone();
        	self.supply()._mint( &Address::zero(), &BigUint::from(1000u64) ); // permanently lock the first MINIMUM_LIQUIDITY tokens 
		} else {
			liquidity = self.minimum(
						(amount_a.clone() * total_supply.clone()) / reserve_a.clone(),
						(amount_b.clone() * total_supply) / reserve_b.clone(),
			);
		}

		require!(liquidity > 0, "Pair: INSUFFICIENT_LIQUIDITY_MINTED");
		self.supply()._mint(&caller, &liquidity);

		reserve_a += amount_a;
		reserve_b += amount_b;

		self.set_pair_reserve(&token_a, &reserve_a);
		self.set_pair_reserve(&token_b, &reserve_b);

        Ok(())
    }

    // https://github.com/Uniswap/uniswap-v2-periphery/blob/dda62473e2da448bc9cb8f4514dadda4aeede5f4/contracts/UniswapV2Router02.sol#L33
	fn _add_liquidity(&self,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint) -> (BigUint, BigUint) {
		// TODO: Add functionality to calculate the amounts for tokens to be sent
		// to liquidity pool
        let reserve_a = self.get_pair_reserve(&self.get_token_a_name());
        let reserve_b = self.get_pair_reserve(&self.get_token_b_name());

        if reserve_a == 0 && reserve_b == 0 {
			return (amount_a_desired, amount_b_desired);
		}

		let tmp = (reserve_a.clone(), reserve_b.clone());
		let amount_b_optimal = self.quote(amount_a_desired.clone(), tmp);
		if amount_b_optimal <= amount_b_desired {
			// assert!(amount_b_optimal > amount_b_min, "Router: INSUFFICIENT_B_AMOUNT");
			return (amount_a_desired, amount_b_optimal);
		} else {
			let tmp = (reserve_b.clone(), reserve_a.clone());
			let amount_a_optimal = self.quote(amount_b_desired.clone(), tmp);
			// assert!(amount_a_optimal <= amount_a_desired);
			// assert!(amount_a_optimal >= amount_a_min, "Router: INSUFFICIENT_A_AMOUNT");
			return (amount_a_optimal, amount_b_desired);
		}
	}

    fn quote(&self, amount_a: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let amount_b = (amount_a * reserves.1) / reserves.0;

        amount_b
	}

	fn get_amount_out(&self, amount_in: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let amount_in_with_fee = amount_in * BigUint::from(997u64);
		let numerator = amount_in_with_fee.clone() * reserves.1;
		let denominator = (reserves.0 * BigUint::from(1000u64)) + amount_in_with_fee;

		let amount_out = numerator / denominator;

        amount_out
	}

	fn get_amount_in(&self, amount_out: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let numerator = (reserves.0 * amount_out.clone()) * BigUint::from(1000u64);
		let denominator = (reserves.1 - amount_out) * BigUint::from(997u64);

		let amount_in = (numerator / denominator) + BigUint::from(1u64);

        amount_in
	}


	fn minimum(&self, value_a: BigUint, value_b: BigUint) -> BigUint {
		if value_a <= value_b {
			return value_a;
		}

		return value_b;
	}

    #[storage_get("token_a_name")]
	fn get_token_a_name(&self) -> TokenIdentifier;

	#[storage_set("token_a_name")]
	fn set_token_a_name(&self, esdt_token_name: &TokenIdentifier);

	#[storage_get("token_b_name")]
	fn get_token_b_name(&self) -> TokenIdentifier;

	#[storage_set("token_b_name")]
	fn set_token_b_name(&self, esdt_token_name: &TokenIdentifier);

    #[view(getReserve)]
    #[storage_get("reserve")]
    fn get_pair_reserve(&self, token: &TokenIdentifier) -> BigUint;

    #[storage_set("reserve")]
    fn set_pair_reserve(&self, token: &TokenIdentifier, balance: &BigUint);

    #[storage_clear("reserve")]
    fn clear_pair_reserve(&self, token: &TokenIdentifier);

}
