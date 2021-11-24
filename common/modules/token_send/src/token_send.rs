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

    fn create_payment(
        &self,
        token_id: &TokenIdentifier,
        nonce: u64,
        amount: &BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        EsdtTokenPayment::new(token_id.clone(), nonce, amount.clone())
    }

    fn nft_create_tokens<T: elrond_codec::TopEncode>(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &T,
    ) -> u64 {
        let mut uris = ManagedVec::new();
        uris.push(self.types().managed_buffer_new());
        self.send().esdt_nft_create::<T>(
            token_id,
            amount,
            &self.types().managed_buffer_new(),
            &BigUint::zero(),
            &self.types().managed_buffer_new(),
            attributes,
            &uris,
        )
    }

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<u64>;
}
