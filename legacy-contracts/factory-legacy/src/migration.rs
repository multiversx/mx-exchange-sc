multiversx_sc::imports!();

use common_structs::UnlockEpochAmountPairs;
use energy_factory::migration::ProxyTrait as _;

#[multiversx_sc::module]
pub trait LockedTokenMigrationModule:
    crate::locked_asset::LockedAssetModule + crate::attr_ex_helper::AttrExHelper
{
    #[only_owner]
    #[endpoint(setNewFactoryAddress)]
    fn set_new_factory_address(&self, sc_address: ManagedAddress) {
        require!(
            !sc_address.is_zero() && self.blockchain().is_smart_contract(&sc_address),
            "Invalid SC address"
        );
        self.new_factory_address().set(&sc_address);
    }

    fn update_energy_after_unlock(
        &self,
        caller: ManagedAddress,
        initial_epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
        final_epoch_amount_pairs: UnlockEpochAmountPairs<Self::Api>,
    ) {
        let new_factory_address = self.new_factory_address().get();
        let _: IgnoreValue = self
            .new_factory_proxy_builder(new_factory_address)
            .update_energy_after_old_token_unlock(
                caller,
                initial_epoch_amount_pairs,
                final_epoch_amount_pairs,
            )
            .execute_on_dest_context();
    }

    #[proxy]
    fn new_factory_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> energy_factory::Proxy<Self::Api>;

    #[storage_mapper("newFactoryAddress")]
    fn new_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
