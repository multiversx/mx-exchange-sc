multiversx_sc::imports!();

use common_structs::Nonce;
use energy_factory::locked_token_transfer::ProxyTrait as _;
use energy_query::Energy;
use simple_lock::locked_token::LockedTokenAttributes;

#[multiversx_sc::module]
pub trait EnergyUpdateModule:
    energy_query::EnergyQueryModule
    + utils::UtilsModule
    + crate::proxy_common::ProxyCommonModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    fn burn_locked_tokens_and_update_energy(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        token_amount: &BigUint,
        user_address: &ManagedAddress,
    ) {
        if token_amount == &0u64 {
            return;
        }

        self.deduct_energy_from_user(user_address, token_id, token_nonce, token_amount);
        self.send()
            .esdt_local_burn(token_id, token_nonce, token_amount);
    }

    fn deduct_energy_from_user(
        &self,
        user: &ManagedAddress,
        token_id: &TokenIdentifier,
        token_nonce: u64,
        token_amount: &BigUint,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_entry(user);

        let new_locked_token_id = self.get_locked_token_id();
        let old_locked_token_id = self.old_locked_token_id().get();
        if token_id == &new_locked_token_id {
            let attributes: LockedTokenAttributes<Self::Api> = self
                .blockchain()
                .get_token_attributes(token_id, token_nonce);
            energy.update_after_unlock_any(token_amount, attributes.unlock_epoch, current_epoch);
        } else if token_id == &old_locked_token_id {
            if self.blockchain().is_smart_contract(user) {
                return;
            }
            let attributes = self.decode_legacy_token(token_id, token_nonce);
            let epoch_amount_pairs = attributes.get_unlock_amounts_per_epoch(token_amount);
            for pair in epoch_amount_pairs.pairs {
                energy.update_after_unlock_any(&pair.amount, pair.epoch, current_epoch);
            }
        } else {
            sc_panic!("Invalid token for energy update");
        }

        let energy_factory_addr = self.energy_factory_address().get();
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
}
