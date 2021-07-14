#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod pair_helper;

#[elrond_wasm_derive::contract]
pub trait DexHelper: pair_helper::PairHelperModule {
    #[init]
    fn init(&self) {}
}
