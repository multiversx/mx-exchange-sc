use elrond_wasm::api::BigUintApi;
use elrond_wasm::{Address, TokenIdentifier};
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub enum Action<BigUint: BigUintApi> {
	AddLiquidity {
		token_a: TokenIdentifier,
		token_b: TokenIdentifier,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		amount_a_min: BigUint,
	    amount_b_min: BigUint,
		caller: Address
	},
	SwapTokens {
        amount_in: BigUint,
        token_name_in: TokenIdentifier,
        amount_out_min: BigUint,
        token_name_out: TokenIdentifier,
        caller: Address
	}
}
