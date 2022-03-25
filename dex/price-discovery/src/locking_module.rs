elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait LockingModule: crate::common_storage::CommonStorageModule {
    #[only_owner]
    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, new_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&new_address),
            "Invalid SC Address"
        );

        self.locking_sc_address().set(&new_address);
    }

    fn lock_tokens_and_forward(
        &self,
        to: ManagedAddress,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) {
        let unlock_epoch = self.unlock_epoch().get();
        let proxy_instance = self.get_locking_sc_proxy_instance();

        proxy_instance
            .lock_tokens(unlock_epoch, OptionalValue::Some(to))
            .add_token_transfer(token_id, 0, amount)
            .execute_on_dest_context_ignore_result();
    }

    fn get_locking_sc_proxy_instance(&self) -> simple_lock::Proxy<Self::Api> {
        let locking_sc_address = self.locking_sc_address().get();
        self.locking_sc_proxy_obj(locking_sc_address)
    }

    #[proxy]
    fn locking_sc_proxy_obj(&self, sc_address: ManagedAddress) -> simple_lock::Proxy<Self::Api>;

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getUnlockEpoch)]
    #[storage_mapper("unlockEpoch")]
    fn unlock_epoch(&self) -> SingleValueMapper<u64>;
}
