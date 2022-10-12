elrond_wasm::imports!();

use common_structs::Energy;
use common_structs::UnlockEpochAmountPairs;

#[elrond_wasm::module]
pub trait SimpleLockMigrationModule:
    crate::energy::EnergyModule + crate::events::EventsModule + elrond_wasm_modules::pause::PauseModule
{
    #[only_owner]
    #[endpoint(updateEnergyForOldTokens)]
    fn update_energy_for_old_tokens(
        &self,
        user: ManagedAddress,
        total_locked_tokens: BigUint,
        energy_amount: BigUint,
    ) {
        self.require_old_tokens_energy_not_updated(&user);

        self.update_energy(&user, |energy: &mut Energy<Self::Api>| {
            energy.add_energy_raw(total_locked_tokens, energy_amount);
        });

        self.user_updated_old_tokens_energy().add(&user);
    }

    #[endpoint(updateEnergyAfterOldTokenUnlock)]
    fn update_energy_after_old_token_unlock(
        &self,
        original_caller: ManagedAddress,
        epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
    ) {
        self.require_caller_old_factory();

        self.update_energy(&original_caller, |energy: &mut Energy<Self::Api>| {
            let current_epoch = self.blockchain().get_block_epoch();
            for pair in epoch_amount_pairs.pairs {
                energy.refund_after_token_unlock(&pair.amount, pair.epoch, current_epoch);
            }
        });
    }

    fn require_caller_old_factory(&self) {
        let caller = self.blockchain().get_caller();
        let old_factory_address = self.old_locked_asset_factory_address().get();
        require!(
            caller == old_factory_address,
            "May only call this through old factory SC"
        );
    }

    fn require_old_tokens_energy_not_updated(&self, address: &ManagedAddress) {
        require!(
            !self.user_updated_old_tokens_energy().contains(address),
            "Energy for old tokens already updated"
        );
    }

    fn require_old_tokens_energy_was_updated(&self, address: &ManagedAddress) {
        require!(
            self.user_updated_old_tokens_energy().contains(address),
            "Must have energy updated for old tokens first"
        );
    }

    #[storage_mapper("oldLockedAssetFactoryAddress")]
    fn old_locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("userUpdatedOldTokensEnergy")]
    fn user_updated_old_tokens_energy(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
