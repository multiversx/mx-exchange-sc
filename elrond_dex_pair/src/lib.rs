#![no_std]

imports!();

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[init]
	fn init(&self, esdt_token_name: TokenIdentifier) {
		self.set_contract_esdt_token_name(&esdt_token_name);
	}

	#[payable("*")]
	#[endpoint(acceptEsdtPayment)]
	fn accept_esdt_payment(
		&self,
		#[payment] esdt_value: BigUint,
		#[payment_token] actual_token_name: TokenIdentifier,
		caller: Address,
	) -> SCResult<()> {
		let expected_token_name = self.get_contract_esdt_token_name();
		require!(actual_token_name == expected_token_name, "Wrong esdt token");

		let mut provider_liquidity = self.get_provider_liquidity(&caller, &expected_token_name);
		provider_liquidity += esdt_value;
		self.set_provider_liquidity(&caller, &expected_token_name, &provider_liquidity);
		Ok(())
	}

	#[endpoint(getReserves)]
	fn get_reserves() -> (reserve0, reserve1, blockTimestampLast)
	{
		// TODO: return
	}

	#[storage_set("esdtTokenName")]
	fn set_contract_esdt_token_name(&self, esdt_token_name: &TokenIdentifier);

	#[view(getEsdtTokenName)]
	#[storage_get("esdtTokenName")]
	fn get_contract_esdt_token_name(&self) -> TokenIdentifier;

	#[storage_get("reserve_a")]
	fn get_reserve_a(&self) -> BigUint;

	#[storage_set("reserve_a")]
	fn set_reserve_a(&self, reserve_a: &BigUint);

	#[storage_get("reserve_b")]
	fn get_reserve_b(&self) -> BigUint;

	#[storage_set("reserve_b")]
	fn set_reserve_b(&self, reserve_a: &BigUint);

	#[view(providerLiquidity)]
	#[storage_get("provider_liquidity")]
	fn get_provider_liquidity(&self, caller: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("provider_liquidity")]
	fn set_provider_liquidity(&self, caller: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);
}