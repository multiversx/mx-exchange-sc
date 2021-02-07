#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use elrond_wasm::HexCallDataSerializer;

#[cfg(feature = "elrond_dex_factory-wasm")]
pub use elrond_dex_factory_wasm as factory;

pub use factory::factory::*;

const PAIR_CONTRACT_ADD_LIQUIDITY: &[u8] = b"acceptEsdtPayment";

#[elrond_wasm_derive::callable(PairProxy)]
pub trait Pair {
	#[callback(get_reserves_callback)]
	fn get_reserves_endpoint(&self,
		#[callback_arg] token_a: TokenIdentifier,
		#[callback_arg] token_b: TokenIdentifier,
		#[callback_arg] amount_a_desired: BigUint,
		#[callback_arg] amount_b_desired: BigUint,
		#[callback_arg] amount_a_min: BigUint,
		#[callback_arg] amount_b_min: BigUint,
		#[callback_arg] caller: Address) -> SCResult< MultiResult2<BigUint, BigUint> >;
	
	fn remove_liquidity(&self, user_address: Address,
		actual_token_a_name: TokenIdentifier,
		actual_token_b_name: TokenIdentifier) -> SCResult<()>;

	fn update_liquidity_provider_storage(&self,
		user_address: Address,
		actual_token_a: TokenIdentifier,
		actual_token_b: TokenIdentifier,
		amount_a: BigUint,
		amount_b: BigUint) -> SCResult<()>;
}

#[elrond_wasm_derive::contract(RouteImpl)]
pub trait Route {

	#[module(FactoryModuleImpl)]
    fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self) {
	}

	#[payable("*")]
	#[endpoint(transferFunds)]
	fn transfer_funds_endpoint(
		&self,
		#[payment] esdt_value: BigUint,
		#[payment_token] token_name: TokenIdentifier,
	) -> SCResult<()> {
		let caller = self.get_caller();

		// To Be moved to web client
		// require!(esdt_value > 100000000000, "Insuficient funds transferred");

		self.set_temporary_funds(&caller, &token_name, &esdt_value);
		Ok(())
	}

	#[endpoint(addLiquidity)]
	fn add_liquidity(
		&self,
		token_a: TokenIdentifier,
		token_b: TokenIdentifier,
		token_a_desired: BigUint,
		token_b_desired: BigUint,
		token_a_min: BigUint,
		token_b_min: BigUint,
	) -> SCResult<()> {
		// TODO: call transfer_liquidity and send to pair smart contract
		// Assumption: User will first send token_a value to this SC
		let caller = self.get_caller();
		let token_a_stored = self.get_temporary_funds(&caller, &token_a);
		let token_b_stored = self.get_temporary_funds(&caller, &token_b);

		require!(token_a_stored > 0, "Insuficient funds A transferred");
		require!(token_b_stored > 0, "Insuficient funds B transferred");

		let pair_address = self.factory().get_pair(&token_a, &token_b);

		let pair_contract = contract_proxy!(self, &pair_address, Pair);
		pair_contract.get_reserves_endpoint(
			token_a,
			token_b,
			token_a_desired,
			token_b_desired,
			token_a_min,
			token_b_min,
			caller,
		);

		Ok(())
	}

	#[endpoint(removeLiquidity)]
	fn remove_liquidity_endpoint(&self,
		token_a: TokenIdentifier,
		token_b: TokenIdentifier) -> SCResult<()> {
		let caller = self.get_caller();
		let pair_address = self.factory().get_pair(&token_a, &token_b);

		let pair_contract = contract_proxy!(self, &pair_address, Pair);
		pair_contract.remove_liquidity(caller, token_a, token_b);

		Ok(())
	}

	// https://github.com/Uniswap/uniswap-v2-periphery/blob/dda62473e2da448bc9cb8f4514dadda4aeede5f4/contracts/UniswapV2Router02.sol#L33
	fn _add_liquidity(&self,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint,
		reserves: (BigUint, BigUint) ) -> (BigUint, BigUint) {
		// TODO: Add functionality to calculate the amounts for tokens to be sent
		// to liquidity pool
		if reserves.0 == 0 && reserves.1 == 0 {
			return (amount_a_desired, amount_b_desired);
	}

		let amount_b_optimal = self.quote(amount_a_desired.clone(), reserves.clone());
		if amount_b_optimal < amount_b_desired {
			assert!(amount_b_optimal > amount_b_min, "Router: INSUFFICIENT_B_AMOUNT");
			return (amount_a_desired, amount_b_optimal);
		} else {
			let amount_a_optimal = self.quote(amount_b_desired.clone(), reserves);
			assert!(amount_a_optimal <= amount_a_desired);
			assert!(amount_a_optimal >= amount_a_min, "Router: INSUFFICIENT_A_AMOUNT");
			return (amount_a_optimal, amount_b_desired);
		}
		}

	fn quote(&self, amount_a: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		assert!(amount_a > 0, "Route: INSUFFICIENT_AMOUNT");
		assert!(reserves.0 > 0, "Route: INSUFFICIENT LIQUIDITY FOR TOKEN A");
		assert!(reserves.1 > 0, "Route: INSUFFICIENT LIQUIDITY FOR TOKEN B");

		(amount_a * reserves.1) / reserves.0
	}

	#[callback]
	fn get_reserves_callback(&self, result: AsyncCallResult< MultiArg2<BigUint, BigUint> >,
								#[callback_arg] token_a: TokenIdentifier,
								#[callback_arg] token_b: TokenIdentifier,
								#[callback_arg] amount_a_desired: BigUint,
								#[callback_arg] amount_b_desired: BigUint,
								#[callback_arg] amount_a_min: BigUint,
								#[callback_arg] amount_b_min: BigUint,
								#[callback_arg] caller: Address) {

		match result {
			AsyncCallResult::Ok(result) => {
				let pair_address = self.factory().get_pair(&token_a, &token_b);
				let reserves = result.into_tuple();
				let (amount_a, amount_b) = self._add_liquidity(
											&token_a,
											&token_b,
											amount_a_desired,
											amount_b_desired,
											amount_a_min,
											amount_b_min,
											reserves);

				self.send().direct_esdt(&pair_address, token_a.as_slice(), &amount_a, &[]);
				self.send().direct_esdt(&pair_address, token_b.as_slice(), &amount_b, &[]);

				let pair_contract = contract_proxy!(self, &pair_address, Pair);
				pair_contract.update_liquidity_provider_storage(caller, token_a, token_b, amount_a, amount_b);
			},
			AsyncCallResult::Err(_) => {},
		}
	}

	// Temporary Storage
	#[view(getTemporaryFunds)]
	#[storage_get("funds")]
	fn get_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("funds")]
	fn set_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);

	#[storage_clear("funds")]
	fn clear_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);
}