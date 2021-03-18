imports!();
derive_imports!();

#[elrond_wasm_derive::module(FeeModuleImpl)]
pub trait FeeModule {
	#[view(getFeeOn)]
	#[storage_get("state")]
	fn get_state(&self) -> bool;

	#[storage_set("enabled")]
	fn set_state(&self, enabled: bool);


	#[view(getFeeToAddress)]
	#[storage_get("fee_to_address")]
	fn get_address(&self) -> Address;

	#[storage_set("fee_to_address")]
	fn set_address(&self, address: &Address);


	#[view(getFeeTokenIdentifier)]
	#[storage_get("fee_token_identifier")]
	fn get_token_identifier(&self) -> TokenIdentifier;

	#[storage_set("fee_token_identifier")]
	fn set_token_identifier(&self, token: &TokenIdentifier);


	#[view(getFeeToFunction)]
	#[storage_get("fee_to_function")]
	fn get_function(&self) -> BoxedBytes;

	#[storage_set("fee_to_function")]
	fn set_function(&self, funtion: &BoxedBytes);
}
