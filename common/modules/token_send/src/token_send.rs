#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use common_errors_old::*;

#[elrond_wasm::module]
pub trait TokenSendModule {
    fn send_multiple_tokens(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: &OptionalValue<ManagedBuffer>,
    ) {
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

        Self::Api::send_api_impl()
            .direct_multi_esdt_transfer_execute(
                destination,
                payments,
                gas_limit,
                &function,
                &ManagedArgBuffer::new_empty(),
            )
            .unwrap_or_else(|_| sc_panic!(ERROR_PAYMENT_FAILED))
    }

    fn send_multiple_tokens_if_not_zero(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: &OptionalValue<ManagedBuffer>,
    ) {
        let mut non_zero_payments = ManagedVec::new();
        for payment in payments.iter() {
            if payment.amount > 0u32 {
                non_zero_payments.push(payment);
            }
        }

        match non_zero_payments.len() {
            0 => {}
            _ => self.send_multiple_tokens(destination, &non_zero_payments, opt_accept_funds_func),
        }
    }

    fn transfer_execute_custom(
        &self,
        to: &ManagedAddress,
        token: &TokenIdentifier,
        nonce: u64,
        amount: &BigUint,
        opt_accept_funds_func: &OptionalValue<ManagedBuffer>,
    ) {
        if amount == &0u32 {
            return;
        }

        let arg_buffer = ManagedArgBuffer::new_empty();
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

        Self::Api::send_api_impl()
            .direct_multi_esdt_transfer_execute(to, &payments, gas_limit, &function, &arg_buffer)
            .unwrap_or_else(|_| sc_panic!(ERROR_PAYMENT_FAILED))
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
        self.send().esdt_nft_create::<T>(
            token_id,
            amount,
            &ManagedBuffer::new(),
            &BigUint::zero(),
            &ManagedBuffer::new(),
            attributes,
            &ManagedVec::new(),
        )
    }

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<u64>;
}
