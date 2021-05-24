#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct FftTokenAmountPair<BigUint: BigUintApi> {
    pub token_id: TokenIdentifier,
    pub amount: BigUint,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct GenericEsdtAmountPair<BigUint: BigUintApi> {
    pub token_id: TokenIdentifier,
    pub token_nonce: Nonce,
    pub amount: BigUint,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenPair {
    pub first_token: TokenIdentifier,
    pub second_token: TokenIdentifier,
}
