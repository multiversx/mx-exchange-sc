#![no_std]

elrond_wasm::imports!();

mod energy_factory {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait EnergyFactory {
        #[view(getEnergyForUser)]
        fn get_energy_for_user(&self, user: ManagedAddress) -> BigUint;
    }
}

#[elrond_wasm::module]
pub trait EnergyQueryModule {
    #[only_owner]
    #[endpoint(setEnergyFactoryAddress)]
    fn set_energy_factory_address(&self, sc_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&sc_address),
            "Invalid address"
        );

        self.energy_factory_address().set(&sc_address);
    }

    fn get_energy_non_zero(&self, user: ManagedAddress) -> BigUint {
        let sc_address = self.energy_factory_address().get();
        let energy: BigUint = self
            .energy_factory_proxy(sc_address)
            .get_energy_for_user(user)
            .execute_on_dest_context();
        require!(energy > 0, "No energy");

        energy
    }

    #[proxy]
    fn energy_factory_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getEnergyFactoryAddress)]
    #[storage_mapper("energyFactoryAddress")]
    fn energy_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
