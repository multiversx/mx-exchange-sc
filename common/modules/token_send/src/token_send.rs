#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

#[elrond_wasm_derive::module]
pub trait TokenSendModule {
    fn send_fft_tokens(
        &self,
        token: &TokenIdentifier,
        amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => (
                accept_funds_func.as_slice(),
                self.transfer_exec_gas_limit().get(),
            ),
            OptionalArg::None => {
                let no_func: &[u8] = &[];
                (no_func, 0u64)
            }
        };

        let _ = self.send().direct_esdt_execute(
            destination,
            token,
            amount,
            gas_limit,
            function,
            &ArgBuffer::new(),
        );
    }

    fn send_nft_tokens(
        &self,
        token: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => (
                accept_funds_func.as_slice(),
                self.transfer_exec_gas_limit().get(),
            ),
            OptionalArg::None => {
                let no_func: &[u8] = &[];
                (no_func, 0u64)
            }
        };

        let _ = self.send().direct_esdt_nft_execute(
            destination,
            token,
            nonce,
            amount,
            gas_limit,
            function,
            &ArgBuffer::new(),
        );
    }

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;
}
