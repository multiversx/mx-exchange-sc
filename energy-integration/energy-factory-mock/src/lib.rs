#![no_std]

elrond_wasm::imports!();

use energy_query_module::Energy;

#[elrond_wasm::contract]
pub trait EnergyFactoryMock {
    #[init]
    fn init(&self) {}

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        self.user_energy(&user).get().get_energy_amount()
    }

    #[view(getEnergyEntryForUser)]
    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;
}
