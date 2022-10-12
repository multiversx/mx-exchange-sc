elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub use common_structs::Energy;

#[elrond_wasm::module]
pub trait EnergyModule: crate::events::EventsModule {
    fn update_energy<T, F: FnOnce(&mut Energy<Self::Api>) -> T>(
        &self,
        user: &ManagedAddress,
        update_fn: F,
    ) -> T {
        let mut energy = self.get_updated_energy_entry_for_user(user);
        let result = update_fn(&mut energy);
        self.set_energy_entry(user, energy);

        result
    }

    fn set_energy_entry(&self, user: &ManagedAddress, new_energy: Energy<Self::Api>) {
        let prev_energy = self.get_updated_energy_entry_for_user(user);

        self.user_energy(user).set(&new_energy);
        self.emit_energy_updated_event(user, prev_energy, new_energy);
    }

    #[view(getEnergyEntryForUser)]
    fn get_updated_energy_entry_for_user(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        let energy_mapper = self.user_energy(user);
        let mut energy = if !energy_mapper.is_empty() {
            energy_mapper.get()
        } else {
            Energy::default()
        };

        let current_epoch = self.blockchain().get_block_epoch();
        energy.deplete(current_epoch);

        energy
    }

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        let energy = self.get_updated_energy_entry_for_user(&user);

        energy.get_energy_amount()
    }

    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;
}
