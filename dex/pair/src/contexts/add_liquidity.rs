elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use crate::State;

pub struct AddLiquidityContext<M: ManagedTypeApi> {
    tx_input: AddLiquidityTxInput<M>,
    contract_state: State,
    lp_token_id: TokenIdentifier<M>,
    first_token_id: TokenIdentifier<M>,
    second_token_id: TokenIdentifier<M>,
}

pub struct AddLiquidityTxInput<M: ManagedTypeApi> {
    args: AddLiquidityArgs<M>,
    payments: AddLiquidityPayments<M>,
}

pub struct AddLiquidityArgs<M: ManagedTypeApi> {
    first_token_amount_min: BigUint<M>,
    second_token_amount_min: BigUint<M>,
    opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
}

pub struct AddLiquidityPayments<M: ManagedTypeApi> {
    first_payment: Option<EsdtTokenPayment<M>>,
    second_payment: Option<EsdtTokenPayment<M>>,
}

impl<M: ManagedTypeApi> AddLiquidityTxInput<M> {
    pub fn new(args: AddLiquidityArgs<M>, payments: AddLiquidityPayments<M>) -> Self {
        AddLiquidityTxInput { args, payments }
    }
}

impl<M: ManagedTypeApi> AddLiquidityArgs<M> {
    pub fn new(
        first_token_amount_min: BigUint<M>,
        second_token_amount_min: BigUint<M>,
        opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
    ) -> Self {
        AddLiquidityArgs {
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        }
    }
}

impl<M: ManagedTypeApi> AddLiquidityPayments<M> {
    pub fn new(
        first_payment: Option<EsdtTokenPayment<M>>,
        second_payment: Option<EsdtTokenPayment<M>>,
    ) -> Self {
        AddLiquidityPayments {
            first_payment,
            second_payment,
        }
    }
}

impl<M: ManagedTypeApi> AddLiquidityContext<M> {
    pub fn new(tx_input: AddLiquidityTxInput<M>) -> Self {
        AddLiquidityContext {
            tx_input,
            contract_state: State::Inactive,
            lp_token_id: TokenIdentifier::egld(),
            first_token_id: TokenIdentifier::egld(),
            second_token_id: TokenIdentifier::egld(),
        }
    }
}

impl<M: ManagedTypeApi> Context<M> for AddLiquidityContext<M> {
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

    fn get_tx_input(&self) -> &dyn TxInput<M> {
        &self.tx_input
    }
}

impl<M: ManagedTypeApi> AddLiquidityContext<M> {}

impl<M: ManagedTypeApi> TxInputArgs<M> for AddLiquidityArgs<M> {
    fn are_valid(&self) -> bool {
        self.first_token_amount_min != 0 && self.second_token_amount_min != 0
    }
}

impl<M: ManagedTypeApi> TxInputPayments<M> for AddLiquidityPayments<M> {
    fn are_valid(&self) -> bool {
        self.is_valid_payment(&self.first_payment.as_ref())
            && self.is_valid_payment(&self.second_payment.as_ref())
    }
}

impl<M: ManagedTypeApi> AddLiquidityPayments<M> {
    fn is_valid_payment(&self, payment_opt: &Option<&EsdtTokenPayment<M>>) -> bool {
        match payment_opt {
            Some(payment) => payment.amount != 0 && payment.token_nonce == 0,
            None => false,
        }
    }
}

impl<M: ManagedTypeApi> TxInput<M> for AddLiquidityTxInput<M> {
    fn get_args(&self) -> &dyn TxInputArgs<M> {
        &self.args
    }

    fn get_payments(&self) -> &dyn TxInputPayments<M> {
        &self.payments
    }

    fn is_valid(&self) -> bool {
        self.args_match_payments()
    }
}

impl<M: ManagedTypeApi> AddLiquidityTxInput<M> {
    fn args_match_payments(&self) -> bool {
        self.min_leq_payment_amount(
            &self.args.first_token_amount_min,
            &self.payments.first_payment.as_ref(),
        ) && self.min_leq_payment_amount(
            &self.args.second_token_amount_min,
            &self.payments.second_payment.as_ref(),
        )
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

impl<M: ManagedTypeApi> AddLiquidityContext<M> {
    pub fn payment_tokens_match_pool_tokens(&self) -> bool {
        self.payment_token_match_pool_token(
            &self.first_token_id,
            &self.tx_input.payments.first_payment.as_ref(),
        ) && self.payment_token_match_pool_token(
            &self.second_token_id,
            &self.tx_input.payments.second_payment.as_ref(),
        )
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
