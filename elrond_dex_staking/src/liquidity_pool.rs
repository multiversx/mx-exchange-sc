imports!();
derive_imports!();

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {

	fn add_liquidity(
		&self,
		amount: BigUint
	) -> SCResult<BigUint> {
		require!(amount > 0, "Amount needs to be greater than 0");

		let mut total_supply = self.total_supply().get();
		let mut virtual_reserves = self.virtual_reserves().get();
		let actual_reserves = self.get_esdt_balance(
			&self.get_sc_address(),
			self.virtual_token_id().get().as_esdt_identifier(),
			0,
		);
		let liquidity: BigUint;

		if total_supply == 0 {
			require!(amount > BigUint::from(1000u64), "First Stake needs to be greater than minimum amount: 1000 * 1000e-18");
			liquidity = amount.clone() - BigUint::from(1000u64);
			total_supply = BigUint::from(1000u64);
			self.total_supply().set(&total_supply);
		} else {
			let total_reserves = virtual_reserves.clone() + actual_reserves.clone();
			liquidity = amount.clone() * total_supply.clone() / total_reserves;
		}
		require!(liquidity > 0, "Insuficient liquidity minted");

		virtual_reserves += amount;
		self.virtual_reserves().set(&virtual_reserves);

		total_supply += liquidity.clone();
		self.total_supply().set(&total_supply);

		Ok(liquidity)
	}

	fn remove_liquidity(
		&self,
		liquidity: BigUint,
		initial_worth: BigUint
	) -> SCResult<BigUint> {
		let reward = sc_try!(self.calculate_reward(liquidity.clone(), initial_worth.clone()));

		let mut virtual_reserves = self.virtual_reserves().get();
		virtual_reserves -= initial_worth;
		self.virtual_reserves().set(&virtual_reserves);

		let mut total_supply = self.total_supply().get();
		total_supply -= liquidity;
		self.total_supply().set(&total_supply);

		Ok(reward)
	}

	fn calculate_reward(
		&self,
		liquidity: BigUint,
		initial_worth: BigUint
	) -> SCResult<BigUint> {
		require!(liquidity > 0, "Liquidity needs to be greater than 0");

		let total_supply = self.total_supply().get();
		require!(total_supply > liquidity, "Removing more liquidity than existent");

		let virtual_reserves = self.virtual_reserves().get();
		require!(virtual_reserves > initial_worth, "Removing more virtual reserve than existent");

		let actual_reserves = self.get_esdt_balance(
			&self.get_sc_address(),
			self.virtual_token_id().get().as_esdt_identifier(),
			0,
		);
		let reward: BigUint;

		let total_reserves = virtual_reserves.clone() + actual_reserves.clone();
		let worth = liquidity.clone() * total_reserves / total_supply.clone();

		if worth > initial_worth {
			reward = worth - initial_worth;
		}
		else {
			reward = BigUint::zero();
		}

		Ok(reward)
	}

	#[view(getTotalSupply)]
	#[storage_mapper("total_supply")]
	fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;

	#[view(getVirtualReserves)]
	#[storage_mapper("virtual_reserves")]
	fn virtual_reserves(&self) -> SingleValueMapper<Self::Storage, BigUint>;

	#[storage_mapper("virtual_token_id")]
	fn virtual_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}

