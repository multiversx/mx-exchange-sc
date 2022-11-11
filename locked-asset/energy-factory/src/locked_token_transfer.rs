elrond_wasm::imports!();

use crate::energy::Energy;

#[elrond_wasm::module]
pub trait LockedTokenTransferModule:
    utils::UtilsModule + crate::energy::EnergyModule + crate::events::EventsModule
{
    #[only_owner]
    #[endpoint(setLockedTokenTransferScAddress)]
    fn set_locked_token_transfer_sc_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);
        self.locked_token_transfer_sc_address().set(&sc_address);
    }

    #[endpoint(setUserEnergyAfterLockedTokenTransfer)]
    fn set_user_energy_after_locked_token_transfer(
        &self,
        user: ManagedAddress,
        energy: Energy<Self::Api>,
    ) {
        let caller = self.blockchain().get_caller();
        let transfer_sc_address = self.locked_token_transfer_sc_address().get();
        require!(
            caller == transfer_sc_address,
            "Only the locked token transfer SC may call this endpoint"
        );

        self.set_energy_entry(&user, energy);
    }

    #[storage_mapper("lockedTokenTransferScAddress")]
    fn locked_token_transfer_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
