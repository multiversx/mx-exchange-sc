#![no_std]

imports!();
derive_imports!();

#[elrond_wasm_derive::contract(FactoryImpl)]
pub trait Factory {

	#[init]
	fn init(&self) {
	}

	#[endpoint]
	fn get_pair_address(&self, token_b: TokenIdentifier) -> SCResult<Address> {
		Ok(self.get_pair(&token_b))
	}

	// Helper function for test
	#[endpoint]
	fn add_pair_contract(&self, token_b: TokenIdentifier, address: &Address) -> SCResult<()> {
		self.set_pair(&token_b, address);
		Ok(())
	}

	#[endpoint(createPair)]
	fn create_pair(&self, token_b: TokenIdentifier) -> SCResult<Address> {
		// TODO: Implement functionality to deploy a new Pair SC
		Ok(Address::zero())
	}

	#[view(getPair)]
	#[storage_get("pair")]
	fn get_pair(&self, token_b: &TokenIdentifier) -> Address;

	#[storage_set("pair")]
	fn set_pair(&self, token_b: &TokenIdentifier, address: &Address);

}