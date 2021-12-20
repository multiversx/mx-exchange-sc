elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use crate::State;

pub struct SwapContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: SwapTxInput<M>,
    storage_cache: StorageCache<M>,
    initial_k: BigUint<M>,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct SwapTxInput<M: ManagedTypeApi> {
    args: SwapArgs<M>,
    payments: SwapPayments<M>,
}

pub struct SwapArgs<M: ManagedTypeApi> {
    opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
}

pub struct SwapPayments<M: ManagedTypeApi> {
    input: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> SwapTxInput<M> {
    pub fn new(args: SwapArgs<M>, payments: SwapPayments<M>) -> Self {
        SwapTxInput { args, payments }
    }
}

impl<M: ManagedTypeApi> SwapArgs<M> {
    pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
        SwapArgs {
            opt_accept_funds_func,
        }
    }
}

impl<M: ManagedTypeApi> SwapPayments<M> {
    pub fn new(input: EsdtTokenPayment<M>) -> Self {
        SwapPayments { input }
    }
}

impl<M: ManagedTypeApi> SwapContext<M> {
    pub fn new(tx_input: SwapTxInput<M>, caller: ManagedAddress<M>) -> Self {
        SwapContext {
            caller,
            tx_input,
            storage_cache: StorageCache::default(),
            initial_k: BigUint::zero(),
            output_payments: ManagedVec::new(),
        }
    }
}

impl<M: ManagedTypeApi> Context<M> for SwapContext<M> {
    fn set_contract_state(&mut self, contract_state: State) {
        self.storage_cache.contract_state = contract_state;
    }

    fn get_contract_state(&self) -> &State {
        &self.storage_cache.contract_state
    }

    fn set_lp_token_id(&mut self, lp_token_id: TokenIdentifier<M>) {
        self.storage_cache.lp_token_id = lp_token_id;
    }

    fn get_lp_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.lp_token_id
    }

    fn set_first_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.storage_cache.first_token_id = token_id;
    }

    fn get_first_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.first_token_id
    }

    fn set_second_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.storage_cache.second_token_id = token_id;
    }

    fn get_second_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.second_token_id
    }

    fn set_first_token_reserve(&mut self, amount: BigUint<M>) {
        self.storage_cache.first_token_reserve = amount;
    }

    fn get_first_token_reserve(&self) -> &BigUint<M> {
        &self.storage_cache.first_token_reserve
    }

    fn set_second_token_reserve(&mut self, amount: BigUint<M>) {
        self.storage_cache.second_token_reserve = amount;
    }

    fn get_second_token_reserve(&self) -> &BigUint<M> {
        &self.storage_cache.second_token_reserve
    }

    fn set_lp_token_supply(&mut self, amount: BigUint<M>) {
        self.storage_cache.lp_token_supply = amount;
    }

    fn get_lp_token_supply(&self) -> &BigUint<M> {
        &self.storage_cache.lp_token_supply
    }

    fn set_initial_k(&mut self, k: BigUint<M>) {
        self.initial_k = k;
    }

    fn get_initial_k(&self) -> &BigUint<M> {
        &self.initial_k
    }

    fn get_caller(&self) -> &ManagedAddress<M> {
        &self.caller
    }

    fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
        self.output_payments = payments
    }

    fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
        &self.output_payments
    }

    fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
        &self.tx_input.args.opt_accept_funds_func
    }

    fn get_tx_input(&self) -> &dyn TxInput<M> {
        &self.tx_input
    }
}

impl<M: ManagedTypeApi> TxInputArgs<M> for SwapArgs<M> {
    fn are_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> TxInputPayments<M> for SwapPayments<M> {
    fn are_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> SwapPayments<M> {
    fn is_valid_payment(&self, payment_opt: &Option<&EsdtTokenPayment<M>>) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> TxInput<M> for SwapTxInput<M> {
    fn get_args(&self) -> &dyn TxInputArgs<M> {
        &self.args
    }

    fn get_payments(&self) -> &dyn TxInputPayments<M> {
        &self.payments
    }

    fn is_valid(&self) -> bool {
        true
    }
}

impl<M: ManagedTypeApi> SwapTxInput<M> {
    fn args_match_payments(&self) -> bool {
        true
    }

    fn min_leq_payment_amount(
        &self,
        min: &BigUint<M>,
        payment_opt: &Option<&EsdtTokenPayment<M>>,
    ) -> bool {
        match payment_opt {
            Some(payment) => min <= &payment.amount,
            None => false,
        }
    }
}

impl<M: ManagedTypeApi> SwapContext<M> {
    pub fn payment_tokens_match_pool_tokens(&self) -> bool {
        true
    }

    fn payment_token_match_pool_token(
        &self,
        token_id: &TokenIdentifier<M>,
        payment_opt: &Option<&EsdtTokenPayment<M>>,
    ) -> bool {
        match payment_opt {
            Some(payment) => token_id == &payment.token_identifier,
            None => false,
        }
    }
}
