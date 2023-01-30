multiversx_sc::imports!();

use energy_factory::virtual_lock::ProxyTrait as _;

#[multiversx_sc::module]
pub trait LockWithEnergyModule {
    #[only_owner]
    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, new_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&new_address),
            "Invalid SC Address"
        );

        self.locking_sc_address().set(&new_address);
    }

    #[only_owner]
    #[endpoint(setLockEpochs)]
    fn set_lock_epochs(&self, lock_epochs: u64) {
        self.lock_epochs().set(lock_epochs);
    }

    fn lock_virtual(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        dest_address: ManagedAddress,
        energy_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        let lock_epochs = self.lock_epochs().get();
        let mut proxy_instance = self.get_locking_sc_proxy_instance();

        proxy_instance
            .lock_virtual(token_id, amount, lock_epochs, dest_address, energy_address)
            .execute_on_dest_context()
    }

    fn get_locking_sc_proxy_instance(&self) -> energy_factory::Proxy<Self::Api> {
        let locking_sc_address = self.locking_sc_address().get();
        self.locking_sc_proxy_obj(locking_sc_address)
    }

    #[proxy]
    fn locking_sc_proxy_obj(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLockEpochs)]
    #[storage_mapper("lockEpochs")]
    fn lock_epochs(&self) -> SingleValueMapper<u64>;
}
