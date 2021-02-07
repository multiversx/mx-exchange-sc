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
}

#[elrond_wasm_derive::contract(RouteImpl)]
pub trait Route {

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
		_token_a: &TokenIdentifier,
		_token_b: &TokenIdentifier,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		_amount_a_min: BigUint,
		_amount_b_min: BigUint,
		_reserves: (BigUint, BigUint) ) -> (BigUint, BigUint) {
		// TODO: Add functionality to calculate the amounts for tokens to be sent
		// to liquidity pool
		(amount_a_desired, amount_b_desired)
	}

	// fn call_esdt_second_contract(
	// 	&self,
	// 	esdt_token_name: &TokenIdentifier,
	// 	amount: &BigUint,
	// 	to: &Address,
	// 	func_name: &[u8],
	// 	args: &[BoxedBytes],
	// ) {
	// 	let mut serializer = HexCallDataSerializer::new(func_name);
	// 	for arg in args {
	// 		serializer.push_argument_bytes(arg.as_slice());
	// 	}

	// 	self.send().direct_esdt(&to, esdt_token_name.as_slice(), amount, serializer.as_slice());
	// }

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