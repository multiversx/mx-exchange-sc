imports!();
derive_imports!();

pub use crate::library::*;

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {

	#[module(LibraryModuleImpl)]
	fn library(&self) -> LibraryModuleImpl<T, BigInt, BigUint>;

	fn mint(
		&self,
		amount_a: BigUint,
		amount_b: BigUint,
	) -> SCResult<BigUint> {
		let token_a = self.get_token_a_name();
		let token_b = self.get_token_b_name();
		let mut total_supply = self.get_total_supply();
		let mut reserve_a = self.get_pair_reserve(&token_a);
		let mut reserve_b = self.get_pair_reserve(&token_b);
		let liquidity: BigUint;
		
		if total_supply == 0 {
			liquidity = self.library().minimum(amount_a.clone(), amount_b.clone()) - BigUint::from(1000u64);
			require!(liquidity > 0, "Pair: FIRST TOKENS NEEDS TO BE GRATER THAN MINIMUM LIQUIDITY: 1000 * e1000-18");
			total_supply += BigUint::from(1000u64);
			self.set_total_supply(&total_supply);
		} else {
			liquidity = self.library().minimum(
						(amount_a.clone() * total_supply.clone()) / reserve_a.clone(),
						(amount_b.clone() * total_supply) / reserve_b.clone(),
			);
		}

		require!(liquidity > 0, "Pair: INSUFFICIENT_LIQUIDITY_MINTED");

		reserve_a += amount_a;
		reserve_b += amount_b;

		self.set_pair_reserve(&token_a, &reserve_a);
		self.set_pair_reserve(&token_b, &reserve_b);

		Ok(liquidity)
	}

	fn burn(
		&self,
		liquidity: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint,
	) -> SCResult<(BigUint, BigUint)> {
		let token_a = self.get_token_a_name();
		let token_b = self.get_token_b_name();
		let mut reserve_a = self.get_pair_reserve(&token_a);
		let mut reserve_b = self.get_pair_reserve(&token_b);

		let total_supply = self.get_total_supply();

		let amount_a = (liquidity.clone() * reserve_a.clone()) / total_supply.clone();
		let amount_b = (liquidity.clone() * reserve_b.clone()) / total_supply;

		require!(&amount_a > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_b > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_a >= &amount_a_min, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_b >= &amount_b_min, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");

		reserve_a -= amount_a.clone();
		reserve_b -= amount_b.clone();

		self.set_pair_reserve(&token_a, &reserve_a);
		self.set_pair_reserve(&token_b, &reserve_b);

		Ok((amount_a, amount_b))
	}

	// https://github.com/Uniswap/uniswap-v2-periphery/blob/dda62473e2da448bc9cb8f4514dadda4aeede5f4/contracts/UniswapV2Router02.sol#L33
	fn _add_liquidity(&self,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint) -> SCResult<(BigUint, BigUint)> {
		// TODO: Add functionality to calculate the amounts for tokens to be sent
		// to liquidity pool
		let reserve_a = self.get_pair_reserve(&self.get_token_a_name());
		let reserve_b = self.get_pair_reserve(&self.get_token_b_name());

		if reserve_a == 0 && reserve_b == 0 {
			return Ok((amount_a_desired, amount_b_desired));
		}

		let tmp = (reserve_a.clone(), reserve_b.clone());
		let amount_b_optimal = self.library().quote(amount_a_desired.clone(), tmp);
		if amount_b_optimal <= amount_b_desired {
			require!(amount_b_optimal > amount_b_min, "PAIR: INSUFFICIENT_B_AMOUNT");
			return Ok((amount_a_desired, amount_b_optimal));
		} else {
			let tmp = (reserve_b.clone(), reserve_a.clone());
			let amount_a_optimal = self.library().quote(amount_b_desired.clone(), tmp);
			require!(amount_a_optimal <= amount_a_desired, "PAIR: OPTIMAL AMOUNT GRATER THAN DESIRED AMOUNT");
			require!(amount_a_optimal >= amount_a_min, "PAIR: INSUFFICIENT_A_AMOUNT");
			return Ok((amount_a_optimal, amount_b_desired));
		}
	}

	fn get_tokens_for_given_position(
		&self,
		liquidity: BigUint
	) -> ((TokenIdentifier, BigUint), (TokenIdentifier, BigUint)) {
		let token_a = self.get_token_a_name();
		let token_b = self.get_token_b_name();
		let reserve_a = self.get_pair_reserve(&token_a);
		let reserve_b = self.get_pair_reserve(&token_b);

		let total_supply = self.get_total_supply();

		let amount_a = (liquidity.clone() * reserve_a.clone()) / total_supply.clone();
		let amount_b = (liquidity.clone() * reserve_b.clone()) / total_supply;

		((token_a, amount_a), (token_b, amount_b))
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

	#[view(getTotalSupply)]
	#[storage_get("total_supply")]
	fn get_total_supply(&self) -> BigUint;

	#[storage_set("total_supply")]
	fn set_total_supply(&self, total_supply: &BigUint);
}
