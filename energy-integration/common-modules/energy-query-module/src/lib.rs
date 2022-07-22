#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

// TODO: Import from LockedAssetFactory after merging with base branch

#[derive(TopEncode, TopDecode)]
pub struct Energy<M: ManagedTypeApi> {
    amount: BigInt<M>,
    last_update_epoch: u64,
    total_locked_tokens: BigUint<M>,
}

mod energy_factory {
    elrond_wasm::imports!();

    use crate::Energy;

    #[elrond_wasm::proxy]
    pub trait EnergyFactory {
        #[view(getEnergyAmountForUser)]
        fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint;

        #[view(getEnergyEntryForUser)]
        fn get_energy_entry_for_user(&self, user: ManagedAddress) -> Energy<Self::Api>;
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

    fn get_energy_amount(&self, user: ManagedAddress) -> BigUint {
        let sc_address = self.energy_factory_address().get();
        self.energy_factory_proxy(sc_address)
            .get_energy_amount_for_user(user)
            .execute_on_dest_context()
    }

    fn get_energy_amount_non_zero(&self, user: ManagedAddress) -> BigUint {
        let energy: BigUint = self.get_energy_amount(user);
        require!(energy > 0, "No energy");

        energy
    }

    fn get_energy_entry(&self, user: ManagedAddress) -> Energy<Self::Api> {
        let sc_address = self.energy_factory_address().get();
        self.energy_factory_proxy(sc_address)
            .get_energy_entry_for_user(user)
            .execute_on_dest_context()
    }

    #[proxy]
    fn energy_factory_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getEnergyFactoryAddress)]
    #[storage_mapper("energyFactoryAddress")]
    fn energy_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
