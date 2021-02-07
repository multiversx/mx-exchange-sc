imports!();

#[elrond_wasm_derive::module(FactoryModuleImpl)]
pub trait FactoryModule {
	#[storage_get("pair")]
	fn get_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> Address;

	#[storage_set("pair")]
	fn set_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier, address: &Address);

	#[storage_is_empty("pair")]
	fn is_empty_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> bool;

	#[storage_get("pair_code")]
	fn get_pair_code(&self) -> BoxedBytes;

	#[storage_set("pair_code")]
	fn set_pair_code(&self, pair_code: &BoxedBytes);

	#[storage_is_empty("pair_code")]
	fn is_empty_pair_code(&self) -> bool;

	fn create_pair(&self, token_a: &TokenIdentifier, token_b: &TokenIdentifier) -> Address {
		if self.is_empty_pair_code() {
			return Address::zero();
		}
		if token_a == token_b {
			return Address::zero();
		}
		let code_metadata : CodeMetadata = CodeMetadata::DEFAULT | CodeMetadata::PAYABLE | CodeMetadata::READABLE;
		let gas_left = self.get_gas_left();
		let amount = BigUint::from(0u32);
		let mut arg_buffer = ArgBuffer::new();
		let code = self.get_pair_code();
		arg_buffer.push_raw_arg(token_a.as_slice());
		arg_buffer.push_raw_arg(token_b.as_slice());
		let new_address = self.send().deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
		self.set_pair(&token_a, &token_b, &new_address);
		self.set_pair(&token_b, &token_a, &new_address);
		new_address
	}
}