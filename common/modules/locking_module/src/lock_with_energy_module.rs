use crate::energy_factory_lock_proxy;

multiversx_sc::imports!();

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
        let locking_sc_address = self.locking_sc_address().get();

        self.tx()
            .to(locking_sc_address)
            .typed(energy_factory_lock_proxy::SimpleLockEnergyProxy)
            .lock_virtual(token_id, amount, lock_epochs, dest_address, energy_address)
            .returns(ReturnsResult)
            .sync_call()
    }

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLockEpochs)]
    #[storage_mapper("lockEpochs")]
    fn lock_epochs(&self) -> SingleValueMapper<u64>;
}
