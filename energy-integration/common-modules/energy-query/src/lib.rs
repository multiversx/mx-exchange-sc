#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub use energy_factory::energy::Energy;

static USER_ENERGY_STORAGE_KEY: &[u8] = b"userEnergy";
static LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"lockedTokenId";
static BASE_TOKEN_ID_STORAGE_KEY: &[u8] = b"baseAssetTokenId";

#[multiversx_sc::module]
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

        let energy_buffer: ManagedBuffer = self.read_energy_from_factory(user);
        if !energy_buffer.is_empty() {
            let mut user_energy: Energy<Self::Api> = Energy::top_decode(energy_buffer)
                .unwrap_or_else(|_| sc_panic!("Failed decoding result from energy factory"));
            user_energy.deplete(current_epoch);

            user_energy
        } else {
            Energy::new_zero_energy(current_epoch)
        }
    }

    fn get_base_token_id(&self) -> TokenIdentifier {
        self.read_raw_storage_from_energy_factory(ManagedBuffer::new_from_bytes(
            BASE_TOKEN_ID_STORAGE_KEY,
        ))
    }

    fn get_locked_token_id(&self) -> TokenIdentifier {
        self.read_raw_storage_from_energy_factory(ManagedBuffer::new_from_bytes(
            LOCKED_TOKEN_ID_STORAGE_KEY,
        ))
    }

    fn read_energy_from_factory<T: TopDecode>(&self, user: &ManagedAddress) -> T {
        let mut key_buffer = ManagedBuffer::new_from_bytes(USER_ENERGY_STORAGE_KEY);
        key_buffer.append(user.as_managed_buffer());

        self.read_raw_storage_from_energy_factory(key_buffer)
    }

    fn read_raw_storage_from_energy_factory<T: TopDecode>(&self, key: ManagedBuffer) -> T {
        let energy_factory_address = self.energy_factory_address().get();
        self.storage_raw()
            .read_from_address(&energy_factory_address, key)
    }

    #[proxy]
    fn energy_factory_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getEnergyFactoryAddress)]
    #[storage_mapper("energyFactoryAddress")]
    fn energy_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
