elrond_wasm::imports!();

use common_structs::PaymentsVec;
use simple_lock::locked_token::LockedTokenAttributes;

use crate::unlock_with_penalty::TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG;

#[elrond_wasm::module]
pub trait LockedTokenTransferModule:
    utils::UtilsModule
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
{
    #[only_owner]
    #[endpoint(setLockedTokenTransferScAddress)]
    fn set_locked_token_transfer_sc_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);
        self.locked_token_transfer_sc_address().set(&sc_address);
    }

    #[endpoint(deductEnergyAfterTransferRegister)]
    fn deduct_energy_after_transfer_register(
        &self,
        from_user: ManagedAddress,
        tokens: PaymentsVec<Self::Api>,
    ) {
        self.require_caller_transfer_sc();

        let current_epoch = self.blockchain().get_block_epoch();
        let locked_token_mapper = self.locked_token();
        self.update_energy(&from_user, |energy| {
            for token in &tokens {
                let attributes: LockedTokenAttributes<Self::Api> =
                    locked_token_mapper.get_token_attributes(token.token_nonce);
                require!(
                    attributes.unlock_epoch > current_epoch,
                    TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
                );

                energy.deplete_after_early_unlock(
                    &token.amount,
                    attributes.unlock_epoch,
                    current_epoch,
                );
            }
        });
    }

    #[endpoint(addEnergyToNewAddressAfterTransfer)]
    fn add_energy_to_new_address_after_transfer(
        &self,
        dest_address: ManagedAddress,
        tokens: PaymentsVec<Self::Api>,
    ) {
        self.require_caller_transfer_sc();

        let current_epoch = self.blockchain().get_block_epoch();
        let locked_token_mapper = self.locked_token();
        self.update_energy(&dest_address, |energy| {
            for token in &tokens {
                let attributes: LockedTokenAttributes<Self::Api> =
                    locked_token_mapper.get_token_attributes(token.token_nonce);
                if attributes.unlock_epoch > current_epoch {
                    energy.add_after_token_lock(
                        &token.amount,
                        attributes.unlock_epoch,
                        current_epoch,
                    );
                } else {
                    // we have to simulate depletion of energy for the new user
                    // otherwise, at unlock time, they would receive free energy
                    // due to the negative energy refund mechanism
                    let epoch_diff = current_epoch - attributes.unlock_epoch;
                    let simulated_deplete_amount = &token.amount * epoch_diff;
                    energy.remove_energy_raw(BigUint::zero(), simulated_deplete_amount);
                }
            }
        });
    }

    fn require_caller_transfer_sc(&self) {
        let caller = self.blockchain().get_caller();
        let transfer_sc_address = self.locked_token_transfer_sc_address().get();
        require!(
            caller == transfer_sc_address,
            "Only the locked token transfer SC may call this endpoint"
        );
    }

    #[storage_mapper("lockedTokenTransferScAddress")]
    fn locked_token_transfer_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
