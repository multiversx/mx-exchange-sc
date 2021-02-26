imports!();

use core::iter::FromIterator;
use elrond_wasm::TokenIdentifier;

#[elrond_wasm_derive::module(FactoryModuleImpl)]
pub trait FactoryModule {

	fn create_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> Address {
		let code_metadata = CodeMetadata::UPGRADEABLE | CodeMetadata::PAYABLE | CodeMetadata::READABLE;
		let gas_left = self.get_gas_left();
		let amount = BigUint::from(0u32);
		let mut arg_buffer = ArgBuffer::new();
		let code = self.get_pair_code();
		arg_buffer.push_raw_arg(token_a.as_slice());
		arg_buffer.push_raw_arg(token_b.as_slice());
		arg_buffer.push_raw_arg(self.get_sc_address().as_bytes());
		let new_address = self.send().deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
		if new_address != Address::zero() {
			self.pair_map_insert((token_a.clone(), token_b.clone()), new_address.clone());
			self.pair_map_insert((token_b.clone(), token_a.clone()), new_address.clone());
		}
		new_address
	}

	fn upgrade_pair(&self, _address: &Address, _new_pair_code: &BoxedBytes) {
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

}