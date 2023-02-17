multiversx_sc::imports!();

use common_structs::PaymentsVec;
use energy_factory::locked_token_transfer::ProxyTrait as _;
use energy_query::Energy;
use simple_lock::locked_token::LockedTokenAttributes;

#[multiversx_sc::module]
pub trait EnergyTransferModule:
    energy_query::EnergyQueryModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    fn deduct_energy_from_sender(
        &self,
        from_user: ManagedAddress,
        tokens: &PaymentsVec<Self::Api>,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_entry(&from_user);
        for token in tokens {
            let attributes: LockedTokenAttributes<Self::Api> = self
                .blockchain()
                .get_token_attributes(&token.token_identifier, token.token_nonce);
            require!(
                attributes.unlock_epoch > current_epoch,
                "Cannot transfer tokens that are unlockable"
            );

            energy.deplete_after_early_unlock(
                &token.amount,
                attributes.unlock_epoch,
                current_epoch,
            );
        }

        self.set_energy_in_factory(from_user, energy);
    }

    fn add_energy_to_destination(&self, to_user: ManagedAddress, tokens: &PaymentsVec<Self::Api>) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_energy_entry(&to_user);
        for token in tokens {
            let attributes: LockedTokenAttributes<Self::Api> = self
                .blockchain()
                .get_token_attributes(&token.token_identifier, token.token_nonce);
            if attributes.unlock_epoch > current_epoch {
                energy.add_after_token_lock(&token.amount, attributes.unlock_epoch, current_epoch);
            } else {
                // we have to simulate depletion of energy for the new user
                // otherwise, at unlock time, they would receive free energy
                // due to the negative energy refund mechanism
                let epoch_diff = current_epoch - attributes.unlock_epoch;
                let simulated_deplete_amount = &token.amount * epoch_diff;
                energy.remove_energy_raw(BigUint::zero(), simulated_deplete_amount);
            }
        }

        self.set_energy_in_factory(to_user, energy);
    }

    fn set_energy_in_factory(&self, user: ManagedAddress, energy: Energy<Self::Api>) {
        let sc_address = self.energy_factory_address().get();
        let _: () = self
            .energy_factory_proxy(sc_address)
            .set_user_energy_after_locked_token_transfer(user, energy)
            .execute_on_dest_context();
    }
}
