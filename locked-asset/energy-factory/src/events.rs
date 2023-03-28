multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::energy::Energy;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct EnergyUpdatedEvent<M: ManagedTypeApi> {
    pub old_energy_entry: Energy<M>,
    pub new_energy_entry: Energy<M>,
}

#[multiversx_sc::module]
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
        self.energy_updated_event(
            user,
            self.blockchain().get_block_nonce(),
            self.blockchain().get_block_epoch(),
            self.blockchain().get_block_timestamp(),
            data,
        );
    }

    #[event("energyUpdated")]
    fn energy_updated_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        data: EnergyUpdatedEvent<Self::Api>,
    );
}
