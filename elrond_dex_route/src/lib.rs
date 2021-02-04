#![no_std]

imports!();
derive_imports!();

use elrond_wasm::HexCallDataSerializer;

const ESDT_TRANSFER_STRING: &[u8] = b"ESDTTransfer";
const PAIR_CONTRACT_ADD_LIQUIDITY: &[u8] = b"acceptEsdtPayment";

#[elrond_wasm_derive::callable(FactoryProxy)]
pub trait Factory {
	#[callback(get_pair_address_callback)]
	fn get_pair_address(&self, token_b: TokenIdentifier,
		#[callback_arg] token_b_identifier: TokenIdentifier,
		#[callback_arg] esdt_value: BigUint,
		#[callback_arg] caller: Address);
}

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[elrond_wasm_derive::contract(RouteImpl)]
pub trait Route {

	#[init]
	fn init(&self, factory_address: &Address) {
		self.set_factory_contract_address(factory_address);
	}

	#[payable("*")]
	#[endpoint(transferToPairContract)]
	fn add_liquidity(
		&self,
		#[payment] esdt_value: BigUint,
		#[payment_token] actual_token_name: TokenIdentifier,
		token_a_desired: BigUint,
		token_b_desired: BigUint,
		token_a_min: BigUint,
		token_b_min: BigUint,
	) -> SCResult<()> {
		// TODO: call transfer_liquidity and send to pair smart contract
		// Assumption: User will first send token_a value to this SC
	}

	// https://github.com/Uniswap/uniswap-v2-periphery/blob/dda62473e2da448bc9cb8f4514dadda4aeede5f4/contracts/UniswapV2Router02.sol#L33
	fn _add_liquidity(token_a: &TokenIdentifier,
					token_b: &TokenIdentifier,
					amount_a_desired: &BigUint,
					amount_b_desired: &BigUint,
					amount_a_min: &BigUint,
					amount_b_min: &BigUint) -> (amount_a: BigUint, amount_b: BigUint) {
		// TODO: Add functionality to calculate the amounts for tokens to be sent
		// to liquidity pool
	}

	// https://github.com/Uniswap/uniswap-v2-periphery/blob/dda62473e2da448bc9cb8f4514dadda4aeede5f4/contracts/UniswapV2Router02.sol#L61
	fn transfer_liquidity(
		token_a: &TokenIdentifier,
		token_b: &TokenIdentifier,
		amount_a_desired: &BigUint,
		amount_b_desired: &BigUint,
		amount_a_min: &BigUint,
		amount_b_min: &BigUint
	) {
		// TODO: Add functionality to send token amounts based on _add_liquidity result
		require!(esdt_value > 0, "no esdt transfered!");

		let factory_address = self.get_factory_contract_address();
		let factory_contract = contract_proxy!(self, &factory_address, Factory);
		factory_contract.get_pair_address(actual_token_name.clone(),
										actual_token_name,
										esdt_value,
										self.get_caller());

		Ok(())

	}

	fn call_esdt_second_contract(
		&self,
		esdt_token_name: &TokenIdentifier,
		amount: &BigUint,
		to: &Address,
		func_name: &[u8],
		args: &[BoxedBytes],
	) {
		let mut serializer = HexCallDataSerializer::new(ESDT_TRANSFER_STRING);
		serializer.push_argument_bytes(esdt_token_name.as_slice());
		serializer.push_argument_bytes(amount.to_bytes_be().as_slice());
		serializer.push_argument_bytes(func_name);
		for arg in args {
			serializer.push_argument_bytes(arg.as_slice());
		}

		self.async_call(&to, &BigUint::zero(), serializer.as_slice());
	}

	#[callback]
	fn get_pair_address_callback(&self, pair_callback: AsyncCallResult<Address>,
								#[callback_arg] token_b_identifier: TokenIdentifier,
								#[callback_arg] esdt_value: BigUint,
								#[callback_arg] caller: Address) {
		match pair_callback {
			AsyncCallResult::Ok(pair_address) => {
				self.set_pair_address(&pair_address);

				self.call_esdt_second_contract(
					&token_b_identifier,
					&esdt_value,
					&pair_address,
					PAIR_CONTRACT_ADD_LIQUIDITY,
					&[BoxedBytes::from(caller.as_bytes())],
				);		
			},
			AsyncCallResult::Err(_) => {},
		}
	}

	#[view(getCallbackCounter)]
	#[storage_get("callback_counter")]
	fn get_callback_counter(&self) -> u32;

	#[storage_set("callback_counter")]
	fn set_callback_counter(&self, counter: u32);

	#[view(GetPairAddress)]
	#[storage_get("pair_address")]
	fn get_pair_address(&self) -> Address;

	#[storage_set("pair_address")]
	fn set_pair_address(&self, pair_address: &Address);

	#[storage_get("factoryContractAddress")]
	fn get_factory_contract_address(&self) -> Address;

	#[storage_set("factoryContractAddress")]
	fn set_factory_contract_address(&self, address: &Address);

	// Temporary Storage
	#[view(getTemporaryFunds)]
	#[storage_get("funds")]
	fn get_temporary_funds(&self, caller: &Address, token_identifier: &BoxedBytes) -> BigUint;

	#[storage_set("funds")]
	fn set_temporary_funds(&self, caller: &Address, token_identifier: &BoxedBytes, amount: &BigUint);
}