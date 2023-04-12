elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::LockedAssetTokenAttributesEx;
use factory::attr_ex_helper;

use crate::{energy::Energy, proxy_common};

static LEGACY_LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"legacyLockedTokenId";
static USER_ENERGY_STORAGE_KEY: &[u8] = b"userEnergy";
static EXTENDED_ATTRIBUTES_ACTIVATION_NONCE_KEY: &[u8] = b"extended_attributes_activation_nonce";

mod energy_factory_proxy {
    elrond_wasm::imports!();
    use crate::energy_update::Energy;

    #[elrond_wasm::proxy]
    pub trait LockedTokenTransferModule {
        #[endpoint(setUserEnergyAfterLockedTokenTransfer)]
        fn set_user_energy_after_locked_token_transfer(
            &self,
            user: ManagedAddress,
            energy: Energy<Self::Api>,
        );
    }
}

#[elrond_wasm::module]
pub trait EnergyUpdateModule:
    proxy_common::ProxyCommonModule + attr_ex_helper::AttrExHelper
{
    #[only_owner]
    #[endpoint(setEnergyFactoryAddress)]
    fn set_energy_factory_address(&self, sc_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&sc_address),
            "Invalid address"
        );

        self.energy_factory_address().set(&sc_address);
    }

    fn deduct_energy_from_user(
        &self,
        user: &ManagedAddress,
        token_id: &TokenIdentifier,
        token_nonce: u64,
        token_amount: &BigUint,
    ) {
        if self.blockchain().is_smart_contract(user) {
            return;
        }

        let energy_factory_addr = self.energy_factory_address().get();
        let legacy_locked_token_id = self.get_legacy_locked_token_id(&energy_factory_addr);
        if token_id != &legacy_locked_token_id {
            return;
        }

        let mut energy = self.get_energy_entry(user);
        let current_epoch = self.blockchain().get_block_epoch();
        let extended_attributes_activation_nonce = self.get_extended_attributes_activation_nonce();
        let attributes: LockedAssetTokenAttributesEx<Self::Api> =
            self.get_attributes_ex(token_id, token_nonce, extended_attributes_activation_nonce);
        let amounts_per_epoch = attributes.get_unlock_amounts_per_epoch(token_amount);
        for epoch_amount_pair in &amounts_per_epoch.pairs {
            energy.update_after_unlock_any(
                &epoch_amount_pair.amount,
                epoch_amount_pair.epoch,
                current_epoch,
            );
        }

        self.set_energy_in_factory(user.clone(), energy, energy_factory_addr);
    }

    fn set_energy_in_factory(
        &self,
        user: ManagedAddress,
        energy: Energy<Self::Api>,
        energy_factory_addr: ManagedAddress,
    ) {
        let _: () = self
            .energy_factory_proxy(energy_factory_addr)
            .set_user_energy_after_locked_token_transfer(user, energy)
            .execute_on_dest_context();
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

    fn get_legacy_locked_token_id(&self, energy_factory_addr: &ManagedAddress) -> TokenIdentifier {
        self.storage_raw().read_from_address(
            energy_factory_addr,
            ManagedBuffer::new_from_bytes(LEGACY_LOCKED_TOKEN_ID_STORAGE_KEY),
        )
    }

    fn get_extended_attributes_activation_nonce(&self) -> u64 {
        let sc_address = self.locked_asset_factory_address().get();
        self.storage_raw().read_from_address(
            &sc_address,
            ManagedBuffer::new_from_bytes(EXTENDED_ATTRIBUTES_ACTIVATION_NONCE_KEY),
        )
    }

    #[proxy]
    fn energy_factory_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> energy_factory_proxy::Proxy<Self::Api>;

    #[view(getEnergyFactoryAddress)]
    #[storage_mapper("energyFactoryAddress")]
    fn energy_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
