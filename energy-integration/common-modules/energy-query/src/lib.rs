#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub use energy_factory::energy::Energy;

static USER_ENERGY_STORAGE_KEY: &[u8] = b"userEnergy";

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

    fn get_energy_amount(&self, user: &ManagedAddress) -> BigUint {
        let user_energy = self.get_energy_entry(user);
        sc_print!("get_energy_amount = {}", user_energy.get_energy_amount());
        user_energy.get_energy_amount()
    }

    fn get_energy_amount_non_zero(&self, user: &ManagedAddress) -> BigUint {
        let energy = self.get_energy_amount(user);
        require!(energy > 0, "No energy");

        energy
    }

    fn get_energy_entry(&self, user: &ManagedAddress) -> Energy<Self::Api> {
        let current_epoch = self.blockchain().get_block_epoch();
        if self.energy_factory_address().is_empty() {
            return Energy::new_zero_energy(current_epoch);
        }

        let energy_buffer: ManagedBuffer = self.read_storage_from_energy_factory(user);
        if !energy_buffer.is_empty() {
            let mut user_energy: Energy<Self::Api> = Energy::top_decode(energy_buffer)
                .unwrap_or_else(|_| sc_panic!("Failed decoding result from energy factory"));
            user_energy.deplete(current_epoch);

            user_energy
        } else {
            Energy::new_zero_energy(current_epoch)
        }
    }

    fn read_storage_from_energy_factory<T: TopDecode>(&self, user: &ManagedAddress) -> T {
        let energy_factory_address = self.energy_factory_address().get();
        let mut key_buffer = ManagedBuffer::new_from_bytes(USER_ENERGY_STORAGE_KEY);
        key_buffer.append(user.as_managed_buffer());

        self.storage_raw()
            .read_from_address(&energy_factory_address, key_buffer)
    }

    #[proxy]
    fn energy_factory_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getEnergyFactoryAddress)]
    #[storage_mapper("energyFactoryAddress")]
    fn energy_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
