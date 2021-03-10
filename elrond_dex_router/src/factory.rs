imports!();

use core::iter::FromIterator;

#[elrond_wasm_derive::module(FactoryModuleImpl)]
pub trait FactoryModule {

	fn init(&self) {
		self.set_pair_code_ready(false);
		self.set_pair_code(&BoxedBytes::empty());
	}

	fn create_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> Address {
		if self.get_pair_code_ready() == false {
			return Address::zero()
		}
		let code_metadata = CodeMetadata::UPGRADEABLE | CodeMetadata::PAYABLE | CodeMetadata::READABLE;
		let gas_left = self.get_gas_left();
		let amount = BigUint::from(0u32);
		let mut arg_buffer = ArgBuffer::new();
		let code = self.get_pair_code();
		arg_buffer.push_argument_bytes(token_a.as_slice());
		arg_buffer.push_argument_bytes(token_b.as_slice());
		arg_buffer.push_argument_bytes(self.get_sc_address().as_bytes());
		let new_address = self.send().deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
		if new_address != Address::zero() {
			self.pair_map_insert((token_a.clone(), token_b.clone()), new_address.clone());
		}
		new_address
	}

	fn start_pair_construct(&self) {
		self.set_pair_code_ready(false);
		self.set_pair_code(&BoxedBytes::empty());
	} 

	fn end_pair_construct(&self) {
		self.set_pair_code_ready(true);
	}

	fn append_pair_code(&self, part: &BoxedBytes) {
		let existent = self.get_pair_code();
		let new_code = BoxedBytes::from_concat(&[existent.as_slice(), part.as_slice()]);
		self.set_pair_code(&new_code);
	}

	fn upgrade_pair(&self, _address: &Address) {
		//TODO
	}

	#[storage_mapper("pair_map")]
	fn pair_map(&self) -> MapMapper<Self::Storage, (TokenIdentifier, TokenIdentifier), Address>;

	fn pair_map_values(&self) -> MultiResultVec<Address> {
		MultiResultVec::from_iter(self.pair_map().values())
	}

	fn pair_map_insert(&self, item: (TokenIdentifier, TokenIdentifier), value: Address) -> Option<Address> {
		let mut pair_map = self.pair_map();
		pair_map.insert(item, value)
	}

	fn pair_map_contains_key(&self, item: (TokenIdentifier, TokenIdentifier)) -> bool {
		let pair_map = self.pair_map();
		pair_map.contains_key(&item)
	}

	fn pair_map_get(&self, item: (TokenIdentifier, TokenIdentifier)) -> Option<Address> {
		let pair_map = self.pair_map();
		pair_map.get(&item)
	}

	#[storage_get("pair_code")]
	fn get_pair_code(&self) -> BoxedBytes;

	#[storage_set("pair_code")]
	fn set_pair_code(&self, pair_code: &BoxedBytes);

	#[storage_get("pairCodeReady")]
	fn get_pair_code_ready(&self) -> bool;

	#[storage_set("pairCodeReady")]
	fn set_pair_code_ready(&self, started: bool);

}