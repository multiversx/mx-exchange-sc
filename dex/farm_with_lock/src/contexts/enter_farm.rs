elrond_wasm::imports!();
elrond_wasm::derive_imports!();

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
    first_payment: EsdtTokenPayment<M>,
    additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

impl<M: ManagedTypeApi> EnterFarmTxInput<M> {
    pub fn new(args: EnterFarmArgs<M>, payments: EnterFarmPayments<M>) -> Self {
        EnterFarmTxInput { args, payments }
    }
}

impl<M: ManagedTypeApi> EnterFarmArgs<M> {
    pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
        EnterFarmArgs {
            opt_accept_funds_func,
        }
    }
}

impl<M: ManagedTypeApi> EnterFarmPayments<M> {
    pub fn new(
        first_payment: EsdtTokenPayment<M>,
        additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
    ) -> Self {
        EnterFarmPayments {
            first_payment,
            additional_payments,
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

    #[inline]
    fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
        self.storage_cache.farm_token_id = farm_token_id
    }

    #[inline]
    fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farm_token_id
    }

    #[inline]
    fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
        self.storage_cache.farming_token_id = farming_token_id
    }

    #[inline]
    fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farming_token_id
    }

    #[inline]
    fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
        self.storage_cache.reward_token_id = reward_token_id;
    }

    #[inline]
    fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.reward_token_id
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

impl<M: ManagedTypeApi> EnterFarmContext<M> {
    pub fn is_accepted_payment(&self) -> bool {
        let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
            == self.storage_cache.farming_token_id
            && self.tx_input.payments.first_payment.token_nonce == 0
            && self.tx_input.payments.first_payment.amount != 0u64;

        if !first_payment_pass {
            return false;
        }

        for payment in self.tx_input.payments.additional_payments.iter() {
            let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                && payment.token_nonce != 0
                && payment.amount != 0;

            if !payment_pass {
                return false;
            }
        }

        true
    }
}
