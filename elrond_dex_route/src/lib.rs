#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[cfg(feature = "elrond_dex_factory-wasm")]
pub use elrond_dex_factory_wasm as factory;

pub use factory::factory::*;


#[elrond_wasm_derive::contract(RouteImpl)]
pub trait Route {

	#[module(FactoryModuleImpl)]
    fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, pair_code: BoxedBytes) {
		self.factory().set_pair_code(&pair_code);
	}

	#[endpoint]
	fn add_pair_address(&self, token_a: TokenIdentifier, token_b: TokenIdentifier, address: Address) -> SCResult<()> {
		self.factory().set_pair(&token_a, &token_b, &address);
		Ok(())
	}

}