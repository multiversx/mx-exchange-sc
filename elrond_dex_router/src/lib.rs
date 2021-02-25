#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod action;
use action::Action;
// use elrond_wasm::HexCallDataSerializer;

#[cfg(feature = "elrond_dex_factory-wasm")]
pub use elrond_dex_factory_wasm as factory;

pub use factory::factory::*;

// const PAIR_CONTRACT_ADD_LIQUIDITY: &[u8] = b"acceptEsdtPayment";

#[elrond_wasm_derive::callable(PairProxy)]
pub trait Pair {
	#[callback(get_reserves_callback)]
	fn get_reserves_endpoint(&self,
		#[callback_arg] action: Action<BigUint>) -> SCResult< MultiResult2<BigUint, BigUint> >;
	
	fn remove_liquidity(&self, user_address: Address,
		actual_token_a_name: TokenIdentifier,
		actual_token_b_name: TokenIdentifier) -> SCResult<()>;

	fn update_liquidity_provider_storage(&self,
		user_address: Address,
		actual_token_a: TokenIdentifier,
		actual_token_b: TokenIdentifier,
		amount_a: BigUint,
		amount_b: BigUint) -> SCResult<()>;

	fn send_tokens_on_swap_success(&self,
		address: Address,
		token_in: TokenIdentifier,
		amount_in: BigUint,
		token_out: TokenIdentifier,
		amount_out: BigUint) -> SCResult<()>;
}

#[elrond_wasm_derive::contract(RouterImpl)]
pub trait Router {

	#[module(FactoryModuleImpl)]
    fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, pair_code: BoxedBytes) {
		self.factory().set_pair_code(&pair_code);
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
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint,
	) -> SCResult<()> {
		// TODO: call transfer_liquidity and send to pair smart contract
		// Assumption: User will first send token_a value to this SC
		let caller = self.get_caller();
		let amount_a_stored = self.get_temporary_funds(&caller, &token_a);
		let amount_b_stored = self.get_temporary_funds(&caller, &token_b);

		require!(amount_a_stored > 0, "Insuficient funds A transferred");
		require!(amount_b_stored > 0, "Insuficient funds B transferred");

		let pair_address;
		if self.factory().is_empty_pair(&token_a, &token_b) {
			pair_address = self.factory().create_pair(&token_a, &token_b);
		}
		else {
			pair_address = self.factory().get_pair(&token_a, &token_b);
		}

		let pair_contract = contract_proxy!(self, &pair_address, Pair);
		pair_contract.get_reserves_endpoint(Action::AddLiquidity {
			token_a,
			token_b,
			amount_a_desired,
			amount_b_desired,
			amount_a_min,
			amount_b_min,
			caller,
		});

		Ok(())
	}

	#[payable("*")]
	#[endpoint(swapToken)]
	fn swap_token_endpoint(
		&self,
		#[payment] amount_in: BigUint,
		#[payment_token] token_name_in: TokenIdentifier,
		amount_out_min: BigUint,
		token_name_out: TokenIdentifier
	) -> SCResult<()> {
		require!(amount_in != 0, "Amount in is zero");

		if token_name_in == token_name_out {
			self.send().direct_esdt(&self.get_caller(), token_name_in.as_slice(), &amount_in, &[]);
			return sc_error!("Can only swap with different tokens");
		}

		if self.factory().is_empty_pair(&token_name_in, &token_name_out) {
			self.send().direct_esdt(&self.get_caller(), token_name_in.as_slice(), &amount_in, &[]);
			return sc_error!("No SC found for this pair");
		}

		let caller = self.get_caller();
		let pair_address = self.factory().get_pair(&token_name_in, &token_name_out);
		let pair_contract = contract_proxy!(self, &pair_address, Pair);
		pair_contract.get_reserves_endpoint(Action::SwapTokens {
			amount_in,
			token_name_in,
			amount_out_min,
			token_name_out,
			caller,
		});

		Ok(())
	}

	#[endpoint(removeLiquidity)]
	fn remove_liquidity_endpoint(&self,
		token_a: TokenIdentifier,
		token_b: TokenIdentifier) -> SCResult<()> {
		let caller = self.get_caller();
		require!(self.factory().is_empty_pair(&token_a, &token_b) == false, "Pair not created");
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
								#[callback_arg] action: Action<BigUint>) {

		match result {
			AsyncCallResult::Ok(result) => {
				match action {
					Action::AddLiquidity {
						token_a,
						token_b,
						amount_a_desired,
						amount_b_desired,
						amount_a_min,
						amount_b_min,
						caller,
					} => {
						let pair_address = self.factory().get_pair(&token_a, &token_b);
						let reserves = result.into_tuple();
						let (amount_a, amount_b) = self._add_liquidity(
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
					Action::SwapTokens {
						amount_in,
						token_name_in,
						amount_out_min,
						token_name_out,
						caller,
					} => {
						let pair_address = self.factory().get_pair(&token_name_in, &token_name_out);
						let reserves = result.into_tuple();
						if reserves.1 >= amount_out_min {
							//Introduce AMM Logic. For now, just send amount_out_min.
							let amount_out = amount_out_min;
							//Send Pair SC the amount received by caller.
							self.send().direct_esdt(&pair_address, token_name_in.as_slice(), &amount_in, &[]);
							//Instruct Pair SC to send the caller token_name_out of amount_out.
							let pair_contract = contract_proxy!(self, &pair_address, Pair);
							pair_contract.send_tokens_on_swap_success(caller, token_name_in, amount_in, token_name_out, amount_out);
						}
						else {
							// Not enough tokens in pair. Sending back the tokens received.
							self.send().direct_esdt(&caller, token_name_in.as_slice(), &amount_in, &[]);
						}
					}
				}
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