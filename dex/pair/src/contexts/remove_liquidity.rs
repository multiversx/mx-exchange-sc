elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use crate::State;

pub struct RemoveLiquidityContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: RemoveLiquidityTxInput<M>,
    contract_state: State,
    lp_token_id: TokenIdentifier<M>,
    first_token_id: TokenIdentifier<M>,
    second_token_id: TokenIdentifier<M>,
    first_token_reserve: BigUint<M>,
    second_token_reserve: BigUint<M>,
    lp_token_supply: BigUint<M>,
    initial_k: BigUint<M>,
    first_token_amount_removed: BigUint<M>,
    second_token_amount_removed: BigUint<M>,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct RemoveLiquidityTxInput<M: ManagedTypeApi> {
    args: RemoveLiquidityArgs<M>,
    payments: RemoveLiquidityPayments<M>,
}

pub struct RemoveLiquidityArgs<M: ManagedTypeApi> {
    first_token_amount_min: BigUint<M>,
    second_token_amount_min: BigUint<M>,
    opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
}

pub struct RemoveLiquidityPayments<M: ManagedTypeApi> {
    lp_token_payment: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> RemoveLiquidityTxInput<M> {
    pub fn new(args: RemoveLiquidityArgs<M>, payments: RemoveLiquidityPayments<M>) -> Self {
        RemoveLiquidityTxInput { args, payments }
    }
}

impl<M: ManagedTypeApi> RemoveLiquidityArgs<M> {
    pub fn new(
        first_token_amount_min: BigUint<M>,
        second_token_amount_min: BigUint<M>,
        opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
    ) -> Self {
        RemoveLiquidityArgs {
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        }
    }
}

impl<M: ManagedTypeApi> RemoveLiquidityPayments<M> {
    pub fn new(lp_token_payment: EsdtTokenPayment<M>) -> Self {
        RemoveLiquidityPayments { lp_token_payment }
    }
}

impl<M: ManagedTypeApi> RemoveLiquidityContext<M> {
    pub fn new(tx_input: RemoveLiquidityTxInput<M>, caller: ManagedAddress<M>) -> Self {
        RemoveLiquidityContext {
            caller,
            tx_input,
            contract_state: State::Inactive,
            lp_token_id: TokenIdentifier::egld(),
            first_token_id: TokenIdentifier::egld(),
            second_token_id: TokenIdentifier::egld(),
            first_token_reserve: BigUint::zero(),
            second_token_reserve: BigUint::zero(),
            lp_token_supply: BigUint::zero(),
            initial_k: BigUint::zero(),
            first_token_amount_removed: BigUint::zero(),
            second_token_amount_removed: BigUint::zero(),
            output_payments: ManagedVec::new(),
        }
    }
}

impl<M: ManagedTypeApi> Context<M> for RemoveLiquidityContext<M> {
    fn set_contract_state(&mut self, contract_state: State) {
        self.contract_state = contract_state;
    }

    fn get_contract_state(&self) -> &State {
        &self.contract_state
    }

    fn set_lp_token_id(&mut self, lp_token_id: TokenIdentifier<M>) {
        self.lp_token_id = lp_token_id;
    }

    fn get_lp_token_id(&self) -> &TokenIdentifier<M> {
        &self.lp_token_id
    }

    fn set_first_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.first_token_id = token_id;
    }

    fn get_first_token_id(&self) -> &TokenIdentifier<M> {
        &self.first_token_id
    }

    fn set_second_token_id(&mut self, token_id: TokenIdentifier<M>) {
        self.second_token_id = token_id;
    }

    fn get_second_token_id(&self) -> &TokenIdentifier<M> {
        &self.second_token_id
    }

    fn set_first_token_reserve(&mut self, amount: BigUint<M>) {
        self.first_token_reserve = amount;
    }

    fn get_first_token_reserve(&self) -> &BigUint<M> {
        &self.first_token_reserve
    }

    fn set_second_token_reserve(&mut self, amount: BigUint<M>) {
        self.second_token_reserve = amount;
    }

    fn get_second_token_reserve(&self) -> &BigUint<M> {
        &self.second_token_reserve
    }

    fn set_lp_token_supply(&mut self, amount: BigUint<M>) {
        self.lp_token_supply = amount;
    }

    fn get_lp_token_supply(&self) -> &BigUint<M> {
        &self.lp_token_supply
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

impl<M: ManagedTypeApi> TxInputArgs<M> for RemoveLiquidityArgs<M> {
    fn are_valid(&self) -> bool {
        self.first_token_amount_min != 0 && self.second_token_amount_min != 0
    }
}

impl<M: ManagedTypeApi> TxInputPayments<M> for RemoveLiquidityPayments<M> {
    fn are_valid(&self) -> bool {
        self.is_valid_payment(&self.lp_token_payment)
    }
}

impl<M: ManagedTypeApi> RemoveLiquidityPayments<M> {
    fn is_valid_payment(&self, payment: &EsdtTokenPayment<M>) -> bool {
        payment.amount != 0 && payment.token_nonce == 0
    }
}

impl<M: ManagedTypeApi> TxInput<M> for RemoveLiquidityTxInput<M> {
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

impl<M: ManagedTypeApi> RemoveLiquidityContext<M> {
    pub fn get_lp_token_payment(&self) -> &EsdtTokenPayment<M> {
        &self.tx_input.payments.lp_token_payment
    }

    pub fn get_first_token_amount_min(&self) -> &BigUint<M> {
        &self.tx_input.args.first_token_amount_min
    }

    pub fn get_second_token_amount_min(&self) -> &BigUint<M> {
        &self.tx_input.args.second_token_amount_min
    }

    pub fn set_first_token_amount_removed(&mut self, amount: BigUint<M>) {
        self.first_token_amount_removed = amount;
    }

    pub fn get_first_token_amount_removed(&self) -> &BigUint<M> {
        &self.first_token_amount_removed
    }

    pub fn set_second_token_amount_removed(&mut self, amount: BigUint<M>) {
        self.second_token_amount_removed = amount;
    }

    pub fn get_second_token_amount_removed(&self) -> &BigUint<M> {
        &self.second_token_amount_removed
    }

    pub fn decrease_lp_token_supply(&mut self) {
        self.lp_token_supply -= &self.tx_input.payments.lp_token_payment.amount;
    }

    pub fn decrease_reserves(&mut self) {
        self.first_token_reserve -= &self.first_token_amount_removed;
        self.second_token_reserve -= &self.second_token_amount_removed;
    }
}
