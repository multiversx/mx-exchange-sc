elrond_wasm::imports!();

use crate::contexts::swap::SwapContext;

#[elrond_wasm::module]
pub trait LockingWrapperModule: locking_module::LockingModule {
    #[only_owner]
    #[endpoint(setLockingDeadlineEpoch)]
    fn set_locking_deadline_epoch(&self, new_deadline: u64) {
        self.locking_deadline_epoch().set(&new_deadline);
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

    #[view(getLockingDeadlineEpoch)]
    #[storage_mapper("locking_deadline_epoch")]
    fn locking_deadline_epoch(&self) -> SingleValueMapper<u64>;
}
