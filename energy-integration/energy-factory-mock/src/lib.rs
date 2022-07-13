#![no_std]

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait EnergyFactoryMock {
    #[init]
    fn init(&self) {}

    #[view(getEnergyForUser)]
    #[storage_mapper("energyForUser")]
    fn energy_for_user(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
