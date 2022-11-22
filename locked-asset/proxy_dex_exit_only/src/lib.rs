#![no_std]

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait ProxyDexExitOnly {
    #[init]
    fn init(&self) {}
}
