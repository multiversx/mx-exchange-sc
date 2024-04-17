use crate::energy_factory_lock_proxy::{self, SimpleLockEnergyProxyMethods};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait LockingModule {
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
    #[endpoint(setUnlockEpoch)]
    fn set_unlock_epoch(&self, new_epoch: u64) {
        self.unlock_epoch().set(new_epoch);
    }

    #[inline]
    fn lock_tokens(
        &self,
        token_id: EgldOrEsdtTokenIdentifier,
        amount: BigUint,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        self.lock_common(OptionalValue::None, token_id, amount)
    }

    #[inline]
    fn lock_tokens_and_forward(
        &self,
        to: ManagedAddress,
        token_id: EgldOrEsdtTokenIdentifier,
        amount: BigUint,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        self.lock_common(OptionalValue::Some(to), token_id, amount)
    }

    fn lock_common(
        &self,
        opt_dest: OptionalValue<ManagedAddress>,
        token_id: EgldOrEsdtTokenIdentifier,
        amount: BigUint,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let unlock_epoch = self.unlock_epoch().get();
        let proxy_instance = self.get_locking_sc_proxy_instance();

        let tokens = proxy_instance
            .lock_tokens_endpoint(unlock_epoch, opt_dest)
            .egld_or_single_esdt(&token_id, 0, &amount)
            .returns(ReturnsResult)
            .sync_call();

        EgldOrEsdtTokenPayment::from(tokens)
    }

    fn get_locking_sc_proxy_instance(
        &self,
    ) -> SimpleLockEnergyProxyMethods<TxScEnv<Self::Api>, (), ManagedAddress, ()> {
        let locking_sc_address = self.locking_sc_address().get();
        self.tx()
            .to(locking_sc_address)
            .typed(energy_factory_lock_proxy::SimpleLockEnergyProxy)
    }

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getUnlockEpoch)]
    #[storage_mapper("unlockEpoch")]
    fn unlock_epoch(&self) -> SingleValueMapper<u64>;
}
