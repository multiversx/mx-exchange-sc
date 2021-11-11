#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait TokenSendModule {
    fn send_multiple_tokens(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        let gas_limit: u64;
        let function: ManagedBuffer;
        let accept_funds_func = opt_accept_funds_func.clone().into_option();
        if accept_funds_func.is_some() {
            gas_limit = self.transfer_exec_gas_limit().get();
            function = accept_funds_func.unwrap();
        } else {
            gas_limit = 0u64;
            function = ManagedBuffer::new();
        }

        self.raw_vm_api()
            .direct_multi_esdt_transfer_execute(
                destination,
                payments,
                gas_limit,
                &function,
                &ManagedArgBuffer::new_empty(self.type_manager()),
            )
            .into()
    }

    fn send_multiple_tokens_if_not_zero(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        let mut non_zero_payments = ManagedVec::new();
        for payment in payments.iter() {
            if payment.amount > 0u32 {
                non_zero_payments.push(payment);
            }
        }

        match non_zero_payments.len() {
            0 => Ok(()),
            _ => self.send_multiple_tokens(destination, &non_zero_payments, opt_accept_funds_func),
        }
    }

    fn transfer_execute_custom(
        &self,
        to: &ManagedAddress,
        token: &TokenIdentifier,
        nonce: u64,
        amount: &BigUint,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        if amount == &0u32 {
            return Ok(());
        }

        let arg_buffer = ManagedArgBuffer::new_empty(self.type_manager());
        let mut payments = ManagedVec::new();
        payments.push(EsdtTokenPayment::new(token.clone(), nonce, amount.clone()));

        let gas_limit: u64;
        let function: ManagedBuffer;
        let accept_funds_func = opt_accept_funds_func.clone().into_option();
        if accept_funds_func.is_some() {
            gas_limit = self.transfer_exec_gas_limit().get();
            function = accept_funds_func.unwrap();
        } else {
            gas_limit = 0u64;
            function = ManagedBuffer::new();
        }

        self.raw_vm_api()
            .direct_multi_esdt_transfer_execute(to, &payments, gas_limit, &function, &arg_buffer)
            .into()
    }

    fn get_all_payments_managed_vec(&self) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        self.raw_vm_api().get_all_esdt_transfers()
    }

    fn manage_vec_remove_index(
        &self,
        vec: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        index: usize,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut copy = ManagedVec::new();

        for (idx, el) in vec.iter().enumerate() {
            if idx != index {
                copy.push(el);
            }
        }

        copy
    }

    fn manage_vec_remove_indexes(
        &self,
        vec: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        index1: usize,
        index2: usize,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let mut copy = ManagedVec::new();

        for (idx, el) in vec.iter().enumerate() {
            if idx != index1 && idx != index2 {
                copy.push(el);
            }
        }

        copy
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
