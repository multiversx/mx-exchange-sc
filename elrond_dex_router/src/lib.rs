#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[cfg(feature = "elrond_dex_factory-wasm")]
pub use elrond_dex_factory_wasm as factory;

pub use factory::factory::*;
use elrond_wasm::TokenIdentifier;

#[elrond_wasm_derive::contract(RouterImpl)]
pub trait Router {

	#[module(FactoryModuleImpl)]
    fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, pair_code: BoxedBytes) {
		self.set_owner(&self.get_caller());
		self.factory().set_pair_code(&pair_code);
	}

	//ENDPOINTS
	#[endpoint(createPair)]
	fn create_pair(&self, token_a: TokenIdentifier, token_b: TokenIdentifier) -> SCResult<Address> {
		require!(token_a != token_b, "Identical tokens");
		let existent_pair = self.factory().pair_map_contains_key((token_a.clone(), token_b.clone()));
		require!(existent_pair == false, "Pair already existent");
		Ok(self.factory().create_pair(&token_a, &token_b))
	}

	#[endpoint(upgradePairs)]
	fn upgrade_pairs(&self, pair_code: BoxedBytes) -> SCResult<()> {
		require!(self.get_caller() == self.get_owner(), "Permission denied");

		let addresses = self.factory().pair_map_values();
		for address in addresses.0.into_iter() {
			self.factory().upgrade_pair(&address, &pair_code)
		}
		self.factory().set_pair_code(&pair_code);
		Ok(())
	}

	//VIEWS
	#[view(getPair)]
	fn get_pair(&self, token_a: TokenIdentifier, token_b: TokenIdentifier) -> Option<Address> {
		self.factory().pair_map_get((token_a, token_b))
	}

	#[view(getAllPairs)]
	fn get_all_pairs(&self) -> MultiResultVec<Address> {
		self.factory().pair_map_values()
	}

	//STORAGE
	#[storage_get("owner")]
	fn get_owner(&self) -> Address;

	#[storage_set("owner")]
	fn set_owner(&self, owner: &Address);
}