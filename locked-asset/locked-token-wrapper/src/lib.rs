#![no_std]

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait LockedTokenWrapper {
    #[init]
    fn init(&self) {}
}
