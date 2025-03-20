multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::LockedAssetTokenAttributesEx;
use energy_factory::{energy::Energy, unlocked_token_transfer::ProxyTrait as _};
use factory_legacy::attr_ex_helper;

use crate::proxy_common;

static LEGACY_LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"legacyLockedTokenId";
static EXTENDED_ATTRIBUTES_ACTIVATION_NONCE_KEY: &[u8] = b"extended_attributes_activation_nonce";

#[multiversx_sc::module]
pub trait EnergyUpdateModule:
    proxy_common::ProxyCommonModule + attr_ex_helper::AttrExHelper + energy_query::EnergyQueryModule
{
    fn deduct_energy_from_user(
        &self,
        user: &ManagedAddress,
        token_id: &TokenIdentifier,
        token_nonce: u64,
        token_amount: &BigUint,
    ) {
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
}
