elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use super::base::*;
use crate::State;

pub struct EnterFarmContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: EnterFarmTxInput<M>,
    storage_cache: StorageCache<M>,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct EnterFarmTxInput<M: ManagedTypeApi> {
    args: EnterFarmArgs<M>,
    payments: EnterFarmPayments<M>,
}

pub struct EnterFarmArgs<M: ManagedTypeApi> {
    opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
}

pub struct EnterFarmPayments<M: ManagedTypeApi> {
    _marker: PhantomData<M>,
}

impl<M: ManagedTypeApi> EnterFarmTxInput<M> {
    pub fn new(args: EnterFarmArgs<M>, payments: EnterFarmPayments<M>) -> Self {
        EnterFarmTxInput { args, payments }
    }
}

impl<M: ManagedTypeApi> EnterFarmArgs<M> {
    pub fn new() -> Self {
        EnterFarmArgs {
            opt_accept_funds_func: OptionalArg::None,
        }
    }
}

impl<M: ManagedTypeApi> EnterFarmPayments<M> {
    pub fn new() -> Self {
        EnterFarmPayments {
            _marker: Default::default(),
        }
    }
}

impl<M: ManagedTypeApi> EnterFarmContext<M> {
    pub fn new(tx_input: EnterFarmTxInput<M>, caller: ManagedAddress<M>) -> Self {
        EnterFarmContext {
            caller,
            tx_input,
            storage_cache: StorageCache::default(),
            output_payments: ManagedVec::new(),
        }
    }
}

impl<M: ManagedTypeApi> Context<M> for EnterFarmContext<M> {
    #[inline]
    fn set_contract_state(&mut self, contract_state: State) {
        self.storage_cache.contract_state = contract_state;
    }

    #[inline]
    fn get_contract_state(&self) -> &State {
        &self.storage_cache.contract_state
    }

    #[inline]
    fn get_caller(&self) -> &ManagedAddress<M> {
        &self.caller
    }

    #[inline]
    fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
        self.output_payments = payments
    }

    #[inline]
    fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
        &self.output_payments
    }

    #[inline]
    fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
        &self.tx_input.args.opt_accept_funds_func
    }

    #[inline]
    fn get_tx_input(&self) -> &dyn TxInput<M> {
        &self.tx_input
    }
}

impl<M: ManagedTypeApi> TxInputArgs<M> for EnterFarmArgs<M> {
    fn are_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> TxInputPayments<M> for EnterFarmPayments<M> {
    fn are_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> EnterFarmPayments<M> {}

impl<M: ManagedTypeApi> TxInput<M> for EnterFarmTxInput<M> {
    #[inline]
    fn get_args(&self) -> &dyn TxInputArgs<M> {
        &self.args
    }

    #[inline]
    fn get_payments(&self) -> &dyn TxInputPayments<M> {
        &self.payments
    }

    fn is_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> EnterFarmTxInput<M> {}

impl<M: ManagedTypeApi> EnterFarmContext<M> {}
