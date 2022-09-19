elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::energy::Energy;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct EnergyUpdatedEvent<M: ManagedTypeApi> {
    pub old_energy_entry: Energy<M>,
    pub new_energy_entry: Energy<M>,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_energy_updated_event(
        &self,
        user: &ManagedAddress,
        old_energy_entry: Energy<Self::Api>,
        new_energy_entry: Energy<Self::Api>,
    ) {
        let data = EnergyUpdatedEvent {
            old_energy_entry,
            new_energy_entry,
        };
        self.energy_updated_event(user, data);
    }

    #[event("energyUpdated")]
    fn energy_updated_event(
        &self,
        #[indexed] address: &ManagedAddress,
        data: EnergyUpdatedEvent<Self::Api>,
    );
}
