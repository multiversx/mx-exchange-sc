elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use crate::State;

pub struct SwapContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: SwapTxInput<M>,
    storage_cache: StorageCache<M>,
    initial_k: BigUint<M>,
    final_input_amount: BigUint<M>,
    final_output_amount: BigUint<M>,
    fee_amount: BigUint<M>,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct SwapTxInput<M: ManagedTypeApi> {
    args: SwapArgs<M>,
    payments: SwapPayments<M>,
}

pub struct SwapArgs<M: ManagedTypeApi> {
    pub output_token_id: TokenIdentifier<M>,
    pub output_amount: BigUint<M>,
    opt_accept_funds_func: OptionalValue<ManagedBuffer<M>>,
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
    pub fn new(
        output_token_id: TokenIdentifier<M>,
        output_amount: BigUint<M>,
        opt_accept_funds_func: OptionalValue<ManagedBuffer<M>>,
    ) -> Self {
        SwapArgs {
            output_token_id,
            output_amount,
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
            final_input_amount: BigUint::zero(),
            final_output_amount: BigUint::zero(),
            fee_amount: BigUint::zero(),
            output_payments: ManagedVec::new(),
        }
    }
}

impl<M: ManagedTypeApi> Context<M> for SwapContext<M> {
    #[inline]
    fn set_contract_state(&mut self, contract_state: State) {
        self.storage_cache.contract_state = contract_state;
    }

    #[inline]
    fn get_contract_state(&self) -> &State {
        &self.storage_cache.contract_state
    }

    #[inline]
    fn set_lp_token_id(&mut self, lp_token_id: TokenIdentifier<M>) {
        self.storage_cache.lp_token_id = lp_token_id;
    }

    #[inline]
    fn get_lp_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.lp_token_id
    }

    #[inline]
    fn set_first_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.storage_cache.first_token_id = token_id;
    }

    #[inline]
    fn get_first_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.first_token_id
    }

    #[inline]
    fn set_second_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.storage_cache.second_token_id = token_id;
    }

    #[inline]
    fn get_second_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.second_token_id
    }

    #[inline]
    fn set_first_token_reserve(&mut self, amount: BigUint<M>) {
        self.storage_cache.first_token_reserve = amount;
    }

    #[inline]
    fn get_first_token_reserve(&self) -> &BigUint<M> {
        &self.storage_cache.first_token_reserve
    }

    #[inline]
    fn set_second_token_reserve(&mut self, amount: BigUint<M>) {
        self.storage_cache.second_token_reserve = amount;
    }

    #[inline]
    fn get_second_token_reserve(&self) -> &BigUint<M> {
        &self.storage_cache.second_token_reserve
    }

    #[inline]
    fn set_lp_token_supply(&mut self, amount: BigUint<M>) {
        self.storage_cache.lp_token_supply = amount;
    }

    #[inline]
    fn get_lp_token_supply(&self) -> &BigUint<M> {
        &self.storage_cache.lp_token_supply
    }

    #[inline]
    fn set_initial_k(&mut self, k: BigUint<M>) {
        self.initial_k = k;
    }

    #[inline]
    fn get_initial_k(&self) -> &BigUint<M> {
        &self.initial_k
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
    fn get_opt_accept_funds_func(&self) -> &OptionalValue<ManagedBuffer<M>> {
        &self.tx_input.args.opt_accept_funds_func
    }

    #[inline]
    fn get_tx_input(&self) -> &dyn TxInput<M> {
        &self.tx_input
    }
}

impl<M: ManagedTypeApi> TxInputArgs<M> for SwapArgs<M> {
    fn are_valid(&self) -> bool {
        self.output_amount != 0 && self.output_token_id.is_esdt()
    }
}

impl<M: ManagedTypeApi> TxInputPayments<M> for SwapPayments<M> {
    fn are_valid(&self) -> bool {
        self.input.amount != 0
            && self.input.token_identifier.is_esdt()
            && self.input.token_nonce == 0
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
        self.args.output_token_id != self.payments.input.token_identifier
    }
}

impl<M: ManagedTypeApi> SwapContext<M> {
    pub fn input_tokens_match_pool_tokens(&self) -> bool {
        (self.tx_input.args.output_token_id == self.storage_cache.first_token_id
            || self.tx_input.args.output_token_id == self.storage_cache.second_token_id)
            && (self.tx_input.payments.input.token_identifier == self.storage_cache.first_token_id
                || self.tx_input.payments.input.token_identifier
                    == self.storage_cache.second_token_id)
    }

    #[inline]
    pub fn get_payment(&self) -> &EsdtTokenPayment<M> {
        &self.tx_input.payments.input
    }

    #[inline]
    pub fn get_swap_args(&self) -> &SwapArgs<M> {
        &self.tx_input.args
    }

    #[inline]
    pub fn get_token_in(&self) -> &TokenIdentifier<M> {
        &self.tx_input.payments.input.token_identifier
    }

    #[inline]
    pub fn get_amount_in(&self) -> &BigUint<M> {
        &self.tx_input.payments.input.amount
    }

    #[inline]
    pub fn get_token_out(&self) -> &TokenIdentifier<M> {
        &self.tx_input.args.output_token_id
    }

    #[inline]
    pub fn get_amount_out_min(&self) -> &BigUint<M> {
        self.get_amount_out()
    }

    #[inline]
    pub fn get_amount_in_max(&self) -> &BigUint<M> {
        self.get_amount_in()
    }

    #[inline]
    pub fn get_amount_out(&self) -> &BigUint<M> {
        &self.tx_input.args.output_amount
    }

    pub fn get_reserve_in(&self) -> &BigUint<M> {
        let payment_token_id = &self.tx_input.payments.input.token_identifier;

        if payment_token_id == &self.storage_cache.first_token_id {
            &self.storage_cache.first_token_reserve
        } else if payment_token_id == &self.storage_cache.second_token_id {
            &self.storage_cache.second_token_reserve
        } else {
            unreachable!()
        }
    }

    pub fn get_reserve_out(&self) -> &BigUint<M> {
        let args_token_id = &self.tx_input.args.output_token_id;

        if args_token_id == &self.storage_cache.first_token_id {
            &self.storage_cache.first_token_reserve
        } else if args_token_id == &self.storage_cache.second_token_id {
            &self.storage_cache.second_token_reserve
        } else {
            unreachable!()
        }
    }

    pub fn increase_reserve_in(&mut self, amount: &BigUint<M>) {
        let payment_token_id = &self.tx_input.payments.input.token_identifier;

        if payment_token_id == &self.storage_cache.first_token_id {
            self.storage_cache.first_token_reserve += amount;
        } else if payment_token_id == &self.storage_cache.second_token_id {
            self.storage_cache.second_token_reserve += amount;
        } else {
            unreachable!()
        }
    }

    pub fn decrease_reserve_out(&mut self, amount: &BigUint<M>) {
        let args_token_id = &self.tx_input.args.output_token_id;

        if args_token_id == &self.storage_cache.first_token_id {
            self.storage_cache.first_token_reserve -= amount;
        } else if args_token_id == &self.storage_cache.second_token_id {
            self.storage_cache.second_token_reserve -= amount;
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub fn set_final_input_amount(&mut self, amount: BigUint<M>) {
        self.final_input_amount = amount
    }

    #[inline]
    pub fn get_final_input_amount(&self) -> &BigUint<M> {
        &self.final_input_amount
    }

    #[inline]
    pub fn set_final_output_amount(&mut self, amount: BigUint<M>) {
        self.final_output_amount = amount
    }

    #[inline]
    pub fn get_final_output_amount(&self) -> &BigUint<M> {
        &self.final_output_amount
    }

    #[inline]
    pub fn set_fee_amount(&mut self, amount: BigUint<M>) {
        self.fee_amount = amount
    }

    #[inline]
    pub fn get_fee_amount(&self) -> &BigUint<M> {
        &self.fee_amount
    }
}
