#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait TokenSendModule {
    fn send_multiple_tokens(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => {
                (accept_funds_func, self.transfer_exec_gas_limit().get())
            }
            OptionalArg::None => {
                let no_func = ManagedBuffer::new();
                (no_func, 0u64)
            }
        };

        let mut final_payments = ManagedVec::new();
        for payment in payments {
            if payment.amount > 0u32 {
                final_payments.push(payment);
            }
        }

        if final_payments.is_empty() {
            return Ok(());
        }

        self.raw_vm_api()
            .direct_multi_esdt_transfer_execute(
                destination,
                &final_payments,
                gas_limit,
                &function,
                &ManagedArgBuffer::new_empty(),
            )
            .into()
    }

    fn transfer_execute_custom(
        &self,
        to: &ManagedAddress,
        token: &TokenIdentifier,
        nonce: u64,
        amount: &BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        if amount == &0u32 {
            return Ok(());
        }

        let (endpoint_name, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => {
                (accept_funds_func, self.transfer_exec_gas_limit().get())
            }
            OptionalArg::None => {
                let no_func = ManagedBuffer::new();
                (no_func, 0u64)
            }
        };
        let arg_buffer = ManagedArgBuffer::new_empty();
        let mut payments = ManagedVec::new();
        payments.push(EsdtTokenPayment::new(token.clone(), nonce, amount.clone()));

        self.raw_vm_api()
            .direct_multi_esdt_transfer_execute(
                to,
                &payments,
                gas_limit,
                &endpoint_name,
                &arg_buffer,
            )
            .into()
    }

    fn create_payment(
        &self,
        token_id: &TokenIdentifier,
        nonce: u64,
        amount: &BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        EsdtTokenPayment::new(token_id.clone(), nonce, amount.clone())
    }

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<u64>;
}
