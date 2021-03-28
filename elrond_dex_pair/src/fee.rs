imports!();
derive_imports!();

#[elrond_wasm_derive::module(FeeModuleImpl)]
pub trait FeeModule {
	#[view(getFeeState)]
	#[storage_mapper("fee_state")]
	fn state(&self) -> SingleValueMapper<Self::Storage, bool>;

	#[view(getFeeToAddress)]
	#[storage_mapper("fee_address")]
	fn address(&self) -> SingleValueMapper<Self::Storage, Address>;

	#[view(getFeeTokenIdentifier)]
	#[storage_mapper("fee_token_identifier")]
	fn token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
