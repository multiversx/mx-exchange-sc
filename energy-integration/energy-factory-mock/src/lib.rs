#![no_std]

elrond_wasm::imports!();

use energy_query::Energy;

#[elrond_wasm::contract]
pub trait EnergyFactoryMock {
    #[init]
    fn init(&self) {}

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        self.get_energy_entry_for_user(user).get_energy_amount()
    }

    #[view(getEnergyEntryForUser)]
    fn get_energy_entry_for_user(&self, user: ManagedAddress) -> Energy<Self::Api> {
        let mapper = self.user_energy(&user);
        if !mapper.is_empty() {
            mapper.get()
        } else {
            Energy::default()
        }
    }

    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;
}
