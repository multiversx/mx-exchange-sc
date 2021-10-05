#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

#[elrond_wasm::module]
pub trait TokenSendModule {
    fn send_fft_tokens(
        &self,
        token: &TokenIdentifier,
        amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => (
                accept_funds_func.clone(),
                self.transfer_exec_gas_limit().get(),
            ),
            OptionalArg::None => (self.types().managed_buffer_new(), 0u64),
        };

        SCResult::from_result(self.raw_vm_api().direct_esdt_execute(
            destination,
            token,
            amount,
            gas_limit,
            &function,
            &ManagedArgBuffer::new_empty(self.type_manager()),
        ))
    }

    fn send_nft_tokens(
        &self,
        token: &TokenIdentifier,
        nonce: Nonce,
        amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => (
                accept_funds_func.clone(),
                self.transfer_exec_gas_limit().get(),
            ),
            OptionalArg::None => (self.types().managed_buffer_new(), 0u64),
        };

        SCResult::from_result(self.raw_vm_api().direct_esdt_nft_execute(
            destination,
            token,
            nonce,
            amount,
            gas_limit,
            &function,
            &ManagedArgBuffer::new_empty(self.type_manager()),
        ))
    }

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<u64>;
}
