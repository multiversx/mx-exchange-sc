#![no_std]

imports!();
derive_imports!();

pub mod liquidity_supply;

pub use crate::liquidity_supply::*;
use core::cmp::min;

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[module(LiquiditySupplyModuleImpl)]
    fn supply(&self) -> LiquiditySupplyModuleImpl<T, BigInt, BigUint>;

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
	#[endpoint]
	fn update_liquidity_provider_storage(&self,
		user_address: Address,
		actual_token_a: TokenIdentifier,
		actual_token_b: TokenIdentifier,
		amount_a: BigUint,
		amount_b: BigUint) -> SCResult<()> {

		let caller = self.get_caller();

		require!(caller == self.get_router_address(), "Permission Denied: Only router has access");
		require!(amount_a > 0 && amount_b > 0, "Invalid tokens amount specified");

		let expected_token_a_name = self.get_token_a_name();
		let expected_token_b_name = self.get_token_b_name();

		require!(actual_token_a == expected_token_a_name, "Wrong token a identifier");
		require!(actual_token_b == expected_token_b_name, "Wrong token b identifier");


		self._update_provider_liquidity(&user_address, &actual_token_a, amount_a);
		self._update_provider_liquidity(&user_address, &actual_token_b, amount_b);
		
		Ok(())
	}

	#[endpoint]
	fn remove_liquidity(&self, user_address: Address,
		actual_token_a_name: TokenIdentifier,
		actual_token_b_name: TokenIdentifier) -> SCResult<()> {
		let caller = self.get_caller();
		require!(caller == self.get_router_address(), "Permission Denied: Only router has access");

		require!(
			user_address != Address::zero(),
			"Can't transfer to default address 0x0!"
		);

		let expected_token_a_name = self.get_token_a_name();
		let expected_token_b_name = self.get_token_b_name();

		require!(actual_token_a_name == expected_token_a_name, "Wrong token a identifier");
		require!(actual_token_b_name == expected_token_b_name, "Wrong token b identifier");

		let amount_a = self.get_provider_liquidity(&user_address, &expected_token_a_name);
		let amount_b = self.get_provider_liquidity(&user_address, &expected_token_b_name);

		self.send().direct_esdt(&user_address, expected_token_a_name.as_slice(), &amount_a, &[]);
		self.send().direct_esdt(&user_address, expected_token_b_name.as_slice(), &amount_b, &[]);

		let mut provider_liquidity = self.get_provider_liquidity(&user_address, &expected_token_a_name);
		provider_liquidity -= amount_a;
		self.set_provider_liquidity(&user_address, &expected_token_a_name, &provider_liquidity);

		let mut provider_liquidity = self.get_provider_liquidity(&user_address, &expected_token_b_name);
		provider_liquidity -= amount_b;
		self.set_provider_liquidity(&user_address, &expected_token_b_name, &provider_liquidity);

		Ok(())
	}

	#[view]
	fn get_reserves_endpoint(&self) -> SCResult< MultiResult2<BigUint, BigUint> > {
		let caller = self.get_caller();
		require!(caller == self.get_router_address(), "Permission Denied: Only router has access");

		let token_a_name = self.get_token_a_name();
		let token_b_name = self.get_token_b_name();

		let reserve_a = self.get_reserve(&token_a_name);
		let reserve_b = self.get_reserve(&token_b_name);

		Ok( (reserve_a, reserve_b).into() )
	}


	fn _update_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier, amount: BigUint) {
		let mut provider_liquidity = self.get_provider_liquidity(user_address, token_identifier);
		provider_liquidity += amount.clone();
		self.set_provider_liquidity(user_address, token_identifier, &provider_liquidity);

		let mut reserve = self.get_reserve(&token_identifier);
		reserve += amount;
		self.set_reserve(&token_identifier, &reserve);
		

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

	#[storage_get("reserve")]
	fn get_reserve(&self, esdt_token_name: &TokenIdentifier) -> BigUint;

	#[storage_set("reserve")]
	fn set_reserve(&self, esdt_token_name: &TokenIdentifier, reserve: &BigUint);

	#[view(providerLiquidity)]
	#[storage_get("provider_liquidity")]
	fn get_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("provider_liquidity")]
	fn set_provider_liquidity(&self, user_address: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);
}