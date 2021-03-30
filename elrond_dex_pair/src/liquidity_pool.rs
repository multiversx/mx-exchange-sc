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
		let token_a = self.token_a_name().get();
		let token_b = self.token_b_name().get();
		let mut total_supply = self.total_supply().get();
		let mut reserve_a = self.get_pair_reserve(&token_a);
		let mut reserve_b = self.get_pair_reserve(&token_b);
		let mut liquidity: BigUint;
		
		if total_supply == 0 {
			liquidity = core::cmp::min(amount_a.clone(), amount_b.clone());
			require!(liquidity > BigUint::from(1000u64), "Pair: FIRST TOKENS NEEDS TO BE GRATER THAN MINIMUM LIQUIDITY: 1000 * 1000e-18");
			liquidity -= BigUint::from(1000u64);
			total_supply += BigUint::from(1000u64);
			self.total_supply().set(&total_supply);
		} else {
			liquidity = core::cmp::min(
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

	fn burn_token(
        &self,
        token: TokenIdentifier,
        liquidity: BigUint,
        total_supply: BigUint,
        amount_min: BigUint,
    ) -> SCResult<BigUint> {
        let mut reserve = self.get_pair_reserve(&token);
        let amount = (liquidity * reserve.clone()) / total_supply;
        require!(&amount > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
        require!(
            &amount >= &amount_min,
            "Pair: INSUFFICIENT_LIQUIDITY_BURNED"
        );
        reserve -= amount.clone();
        self.set_pair_reserve(&token, &reserve);
        Ok(amount)
    }

    fn burn(
        &self,
        liquidity: BigUint,
        amount_a_min: BigUint,
        amount_b_min: BigUint,
    ) -> SCResult<(BigUint, BigUint)> {
        let total_supply = self.total_supply().get();
        let amount_a = sc_try!(self.burn_token(
            self.token_a_name().get(),
            liquidity.clone(),
            total_supply.clone(),
            amount_a_min
        ));
        let amount_b = sc_try!(self.burn_token(
            self.token_b_name().get(),
            liquidity,
            total_supply,
            amount_b_min
        ));
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
		let reserve_a = self.get_pair_reserve(&self.token_a_name().get());
		let reserve_b = self.get_pair_reserve(&self.token_b_name().get());

		if reserve_a == 0 && reserve_b == 0 {
			return Ok((amount_a_desired, amount_b_desired));
		}

		let amount_b_optimal = self.library().quote(amount_a_desired.clone(), reserve_a.clone(), reserve_b.clone());
		if amount_b_optimal <= amount_b_desired {
			require!(amount_b_optimal > amount_b_min, "PAIR: INSUFFICIENT_B_AMOUNT");
			return Ok((amount_a_desired, amount_b_optimal));
		} else {
			let amount_a_optimal = self.library().quote(amount_b_desired.clone(), reserve_b.clone(), reserve_a.clone());
			require!(amount_a_optimal <= amount_a_desired, "PAIR: OPTIMAL AMOUNT GRATER THAN DESIRED AMOUNT");
			require!(amount_a_optimal >= amount_a_min, "PAIR: INSUFFICIENT_A_AMOUNT");
			return Ok((amount_a_optimal, amount_b_desired));
		}
	}

	fn get_token_for_given_position(
		&self,
		liquidity: BigUint,
		token: &TokenIdentifier
	) -> BigUint {
		let reserve = self.get_pair_reserve(&token);
		let total_supply = self.total_supply().get();
		if total_supply != BigUint::zero() {
			liquidity.clone() * reserve.clone() / total_supply.clone()
		}
		else {
			BigUint::zero()
		}
	}

	fn get_tokens_for_given_position(
		&self,
		liquidity: BigUint
	) -> ((TokenIdentifier, BigUint), (TokenIdentifier, BigUint)) {
		let token_a_name = self.token_a_name().get();
		let amount_a = self.get_token_for_given_position(
			liquidity.clone(),
			&token_a_name
		);
		let token_b_name = self.token_b_name().get();
		let amount_b = self.get_token_for_given_position(
			liquidity.clone(),
			&token_b_name
		);
		((token_a_name, amount_a), (token_b_name, amount_b))
	}

	fn calculate_k(&self) -> BigUint {
		let amount_a = self.get_pair_reserve(&self.token_a_name().get());
		let amount_b = self.get_pair_reserve(&self.token_b_name().get());
		self.library().calculate_k(amount_a, amount_b)
	}

	#[view(getTokenAName)]
	#[storage_mapper("token_a_name")]
	fn token_a_name(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

	#[view(getTokenBName)]
	#[storage_mapper("token_b_name")]
	fn token_b_name(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

	#[view(getReserve)]
	#[storage_get("reserve")]
	fn get_pair_reserve(&self, token: &TokenIdentifier) -> BigUint;

	#[storage_set("reserve")]
	fn set_pair_reserve(&self, token: &TokenIdentifier, balance: &BigUint);

	#[storage_clear("reserve")]
	fn clear_pair_reserve(&self, token: &TokenIdentifier);

	#[view(getTotalSupply)]
	#[storage_mapper("total_supply")]
	fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;
}
