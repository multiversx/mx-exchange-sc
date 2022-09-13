#![no_std]

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait SimpleLockEnergy {
    #[init]
    fn init(&self) {}
}
