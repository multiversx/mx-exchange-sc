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
        let mut proxy_instance = self.get_locking_sc_proxy_instance();

        let payment: EgldOrEsdtTokenPayment<Self::Api> = proxy_instance
            .lock_tokens_endpoint(unlock_epoch, opt_dest)
            .with_esdt_transfer((token_id, 0, amount))
            .execute_on_dest_context();
        let (token_id, token_nonce, amount) = payment.into_tuple();

        EsdtTokenPayment::new(token_id.unwrap_esdt(), token_nonce, amount)
    }

    fn should_generate_locked_asset(&self) -> bool {
        let current_epoch = self.blockchain().get_block_epoch();
        let locking_deadline_epoch = self.locking_deadline_epoch().get();

        current_epoch < locking_deadline_epoch
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

    #[view(getLockingDeadlineEpoch)]
    #[storage_mapper("locking_deadline_epoch")]
    fn locking_deadline_epoch(&self) -> SingleValueMapper<u64>;
}
