#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait GlobalOperationModule {
    fn global_op_start(&self) {
        self.global_op_is_ongoing().set(&true);
    }

    fn global_op_stop(&self) {
        self.global_op_is_ongoing().set(&false);
    }

    #[storage_mapper("global_operation_ongoing")]
    fn global_op_is_ongoing(&self) -> SingleValueMapper<Self::Storage, bool>;

    fn require_global_op_not_ongoing(&self) -> SCResult<()> {
        require!(
            !self.global_op_is_ongoing().get(),
            "Global operation ongoing"
        );
        Ok(())
    }

    fn require_global_op_ongoing(&self) -> SCResult<()> {
        require!(
            self.global_op_is_ongoing().get(),
            "Global operation not ongoing"
        );
        Ok(())
    }
}
