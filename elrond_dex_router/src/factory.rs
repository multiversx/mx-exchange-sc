elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct PairKey {
	pub token_a: TokenIdentifier,
	pub token_b: TokenIdentifier,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct PairContractData {
	address: Address,
	token_a: TokenIdentifier,
	token_b: TokenIdentifier
}

#[elrond_wasm_derive::module(FactoryModuleImpl)]
pub trait FactoryModule {

	fn init(&self) {
		self.pair_code_ready().set(&false);
		self.pair_code().set(&BoxedBytes::empty());
	}

	fn create_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> Address {
		if self.pair_code_ready().get() == false {
			return Address::zero()
		}
		let code_metadata = CodeMetadata::UPGRADEABLE;
		let gas_left = self.get_gas_left();
		let amount = BigUint::from(0u32);
		let mut arg_buffer = ArgBuffer::new();
		let code = self.pair_code().get();
		arg_buffer.push_argument_bytes(token_a.as_esdt_identifier());
		arg_buffer.push_argument_bytes(token_b.as_esdt_identifier());
		arg_buffer.push_argument_bytes(self.get_sc_address().as_bytes());
		let new_address = self.send().deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
		if new_address != Address::zero() {
			self.pair_map().insert(
				PairKey{
					token_a: token_a.clone(),
					token_b: token_b.clone(),
				},
				new_address.clone());
		}
		new_address
	}

	fn start_pair_construct(&self) {
		self.pair_code_ready().set(&false);
		self.pair_code().set(&BoxedBytes::empty());
	} 

	fn end_pair_construct(&self) {
		self.pair_code_ready().set(&true);
	}

	fn append_pair_code(&self, part: &BoxedBytes) {
		let existent = self.pair_code().get();
		let new_code = BoxedBytes::from_concat(&[existent.as_slice(), part.as_slice()]);
		self.pair_code().set(&new_code);
	}

	fn upgrade_pair(&self, _address: &Address) {
		//TODO
	}

	#[storage_mapper("pair_map")]
	fn pair_map(&self) -> MapMapper<Self::Storage, PairKey, Address>;

	#[view(getAllPairsAddresses)]
	fn get_all_pairs_addresses(&self) -> MultiResultVec<Address> {
		MultiResultVec::from_iter(self.pair_map().values())
	}

	#[view(getAllPairsTokens)]
	fn get_all_pairs(&self) -> MultiResultVec<(TokenIdentifier, TokenIdentifier)> {
		MultiResultVec::from_iter(self.pair_map().keys())
	}

	#[view(getAllPairs)]
	fn get_pairs(&self) -> MultiResultVec<PairContractData> {
		let map: Vec<PairContractData> = self.pair_map()
			.iter()
			.map(|x| PairContractData {
					address: x.1,
					token_a: x.0.0,
					token_b: x.0.1
				}
			)
			.collect();
		MultiResultVec::from_iter(map)
	}

	#[view(getPairCode)]
	#[storage_mapper("pair_code")]
	fn pair_code(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

	#[view(getPairCodeReady)]
	#[storage_mapper("pair_code_ready")]
	fn pair_code_ready(&self) -> SingleValueMapper<Self::Storage, bool>;
}