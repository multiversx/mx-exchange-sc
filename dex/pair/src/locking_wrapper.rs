use crate::energy_factory_lock_proxy::{self, SimpleLockEnergyProxyMethods};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait LockingWrapperModule:
    crate::config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    #[endpoint(setLockingDeadlineEpoch)]
    fn set_locking_deadline_epoch(&self, new_deadline: u64) {
        self.require_caller_has_owner_permissions();
        self.locking_deadline_epoch().set(new_deadline);
    }

    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, new_address: ManagedAddress) {
        self.require_caller_has_owner_permissions();
        require!(
            self.blockchain().is_smart_contract(&new_address),
            "Invalid SC Address"
        );

        self.locking_sc_address().set(&new_address);
    }

    #[endpoint(setUnlockEpoch)]
    fn set_unlock_epoch(&self, new_epoch: u64) {
        self.require_caller_has_owner_permissions();
        self.unlock_epoch().set(new_epoch);
    }

    #[inline]
    fn lock_tokens(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        self.lock_common(OptionalValue::None, token_id, amount)
    }

    #[inline]
    fn lock_tokens_and_forward(
        &self,
        to: ManagedAddress,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        self.lock_common(OptionalValue::Some(to), token_id, amount)
    }

    fn lock_common(
        &self,
        opt_dest: OptionalValue<ManagedAddress>,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        let unlock_epoch = self.unlock_epoch().get();
        let proxy_instance = self.get_locking_sc_proxy_instance();

        proxy_instance
            .lock_tokens_endpoint(unlock_epoch, opt_dest)
            .single_esdt(&token_id, 0, &amount)
            .returns(ReturnsResult)
            .sync_call()
    }

    fn should_generate_locked_asset(&self) -> bool {
        let current_epoch = self.blockchain().get_block_epoch();
        let locking_deadline_epoch = self.locking_deadline_epoch().get();

        current_epoch < locking_deadline_epoch
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

    #[view(getLockingDeadlineEpoch)]
    #[storage_mapper("locking_deadline_epoch")]
    fn locking_deadline_epoch(&self) -> SingleValueMapper<u64>;
}
