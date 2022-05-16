elrond_wasm::imports!();

use crate::contexts::swap::SwapContext;

#[elrond_wasm::module]
pub trait LockingWrapperModule: crate::config::ConfigModule + token_send::TokenSendModule {
    #[endpoint(setLockingDeadlineEpoch)]
    fn set_locking_deadline_epoch(&self, new_deadline: u64) {
        self.require_permissions();
        self.locking_deadline_epoch().set(&new_deadline);
    }

    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, new_address: ManagedAddress) {
        self.require_permissions();
        require!(
            self.blockchain().is_smart_contract(&new_address),
            "Invalid SC Address"
        );

        self.locking_sc_address().set(&new_address);
    }

    #[endpoint(setUnlockEpoch)]
    fn set_unlock_epoch(&self, new_epoch: u64) {
        self.require_permissions();
        self.unlock_epoch().set(&new_epoch);
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

        proxy_instance
            .lock_tokens(unlock_epoch, opt_dest)
            .add_token_transfer(token_id, 0, amount)
            .execute_on_dest_context()
    }

    fn should_generate_locked_asset(&self) -> bool {
        let current_epoch = self.blockchain().get_block_epoch();
        let locking_deadline_epoch = self.locking_deadline_epoch().get();

        current_epoch < locking_deadline_epoch
    }

    fn call_lock_tokens(&self, context: &SwapContext<Self::Api>) -> EsdtTokenPayment<Self::Api> {
        let token_out = context.get_token_out().clone();
        let amount_out = context.get_final_output_amount().clone();

        self.lock_tokens(token_out, amount_out)
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
