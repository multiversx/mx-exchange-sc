#![no_std]

imports!();

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[init]
	fn init(&self, token_a_name: TokenIdentifier, token_b_name: TokenIdentifier, router_address: Address) {
		self.set_token_a_name(&token_a_name);
		self.set_token_b_name(&token_b_name);
		self.set_router_address(&router_address);
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


	#[storage_get("router_address")]
	fn get_router_address(&self) -> Address;

	#[storage_set("router_address")]
	fn set_router_address(&self, router_address: &Address);

	#[storage_get("token_a_name")]
	fn get_token_a_name(&self) -> TokenIdentifier;

	#[storage_set("token_a_name")]
	fn set_token_a_name(&self, esdt_token_name: &TokenIdentifier);

	#[storage_get("token_b_name")]
	fn get_token_b_name(&self) -> TokenIdentifier;

	#[storage_set("token_b_name")]
	fn set_token_b_name(&self, esdt_token_name: &TokenIdentifier);

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