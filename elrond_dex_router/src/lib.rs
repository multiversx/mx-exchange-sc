#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod factory;
pub use factory::*;


#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
	fn set_fee_on_endpoint(
		&self, 
		enabled: bool, 
		fee_to_address: Address, 
		fee_token: TokenIdentifier
	) -> ContractCall<BigUint>;
}

#[elrond_wasm_derive::contract(RouterImpl)]
pub trait Router {

	#[module(FactoryModuleImpl)]
	fn factory(&self) -> FactoryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self) {
		self.factory().init();
	}

	//ENDPOINTS
	#[endpoint(createPair)]
	fn create_pair(&self, token_a: TokenIdentifier, token_b: TokenIdentifier) -> SCResult<Address> {
		require!(token_a != token_b, "Identical tokens");
		require!(token_a.is_esdt(), "Only esdt tokens allowed");
		require!(token_b.is_esdt(), "Only esdt tokens allowed");
		let existent_pair = self.factory().pair_map_contains_key((token_a.clone(), token_b.clone()));
		require!(existent_pair == false, "Pair already existent");
		Ok(self.factory().create_pair(&token_a, &token_b))
	}

	#[endpoint(setStakingInfo)]
	fn set_staking_info(
		&self, 
		staking_address: Address, 
		staking_token: TokenIdentifier
	) -> SCResult<()> {
		only_owner!(self, "Permission denied");
		self.set_staking_address(&staking_address);
		self.set_staking_token(&staking_token);
		Ok(())
	}

	#[endpoint(upgradePair)]
	fn upgrade_pair(&self, pair_address: Address) -> SCResult<()> {
		only_owner!(self, "Permission denied");

		let addresses = self.factory().pair_map_values();
		let mut found = false;
		for address in addresses.0.into_iter() {
			if address == pair_address {
				found = true;
				break;
			}
		}

		require!(found == true, "Not a pair SC");
		self.factory().upgrade_pair(&pair_address);
		Ok(())
	}

	#[endpoint(setFeeOn)]
	fn set_fee_on(&self, pair_address: Address) -> SCResult<AsyncCall<BigUint>> {
		only_owner!(self, "Permission denied");

		let addresses = self.factory().pair_map_values();
		let mut found = false;
		for address in addresses.0.into_iter() {
			if address == pair_address {
				found = true;
				break;
			}
		}

		require!(found == true, "Not a pair SC");
		let staking_token = self.get_staking_token();
		let staking_address = self.get_staking_address();
		Ok(contract_call!(self, pair_address, PairContractProxy)
			.set_fee_on_endpoint(true, staking_address, staking_token)
			.async_call())
	}

	#[endpoint(setFeeOff)]
	fn set_fee_off(&self, pair_address: Address) -> SCResult<AsyncCall<BigUint>> {
		only_owner!(self, "Permission denied");

		let addresses = self.factory().pair_map_values();
		let mut found = false;
		for address in addresses.0.into_iter() {
			if address == pair_address {
				found = true;
				break;
			}
		}

		require!(found == true, "Not a pair SC");
		Ok(contract_call!(self, pair_address, PairContractProxy)
			.set_fee_on_endpoint(false, Address::zero(), TokenIdentifier::egld())
			.async_call())
	}

	#[endpoint(startPairCodeConstruction)]
	fn start_pair_code_construction(&self) -> SCResult<()> {
		only_owner!(self, "Permission denied");

		self.factory().start_pair_construct();
		Ok(())
	}

	#[endpoint(endPairCodeConstruction)]
	fn end_pair_code_construction(&self) -> SCResult<()> {
		only_owner!(self, "Permission denied");

		self.factory().end_pair_construct();
		Ok(())
	}

	#[endpoint(appendPairCode)]
	fn apppend_pair_code(&self, part: BoxedBytes) -> SCResult<()> {		
		only_owner!(self, "Permission denied");

		self.factory().append_pair_code(&part);
		Ok(())
	}

	//VIEWS
	#[view(getPair)]
	fn get_pair(&self, token_a: TokenIdentifier, token_b: TokenIdentifier) -> SCResult<Address> {
		let mut address = self.factory().pair_map_get((token_a.clone(), token_b.clone())).unwrap_or(Address::zero());
		if address == Address::zero() {
			address = self.factory().pair_map_get((token_b.clone(), token_a.clone())).unwrap_or(Address::zero());
		}
		Ok(address)
	}

	#[view(getAllPairs)]
	fn get_all_pairs(&self) -> MultiResultVec<Address> {
		self.factory().pair_map_values()
	}


	#[view(getStakingAddress)]
	#[storage_get("staking_address")]
	fn get_staking_address(&self) -> Address;

	#[storage_set("staking_address")]
	fn set_staking_address(&self, address: &Address);


	#[view(getStakingToken)]
	#[storage_get("staking_token")]
	fn get_staking_token(&self) -> TokenIdentifier;

	#[storage_set("staking_token")]
	fn set_staking_token(&self, token: &TokenIdentifier);
}