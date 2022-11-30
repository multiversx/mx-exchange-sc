elrond_wasm::imports!();

use energy_factory::locked_token_transfer::ProxyTrait as _;
use energy_query::Energy;
use simple_lock::locked_token::LockedTokenAttributes;

static LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"lockedTokenId";

#[elrond_wasm::module]
pub trait EnergyUpdateModule: energy_query::EnergyQueryModule + utils::UtilsModule {
    fn deduct_energy_from_user(
        &self,
        user: &ManagedAddress,
        token_id: &TokenIdentifier,
        token_nonce: u64,
        token_amount: &BigUint,
    ) {
        let energy_factory_addr = self.energy_factory_address().get();
        let locked_token_id = self.get_locked_token_id(&energy_factory_addr);
        if token_id != &locked_token_id {
            return;
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let attributes: LockedTokenAttributes<Self::Api> =
            self.get_token_attributes(token_id, token_nonce);

        let mut energy = self.get_energy_entry(user);
        energy.update_after_unlock_any(token_amount, attributes.unlock_epoch, current_epoch);
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

    fn get_locked_token_id(&self, energy_factory_addr: &ManagedAddress) -> TokenIdentifier {
        self.storage_raw().read_from_address(
            energy_factory_addr,
            ManagedBuffer::new_from_bytes(LOCKED_TOKEN_ID_STORAGE_KEY),
        )
    }
}
