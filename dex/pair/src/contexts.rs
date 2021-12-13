elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config::State;

pub trait Context<M: ManagedTypeApi> {
    fn set_contract_state(&mut self, contract_state: State);
    fn get_contract_state(&self) -> &State;

    fn set_lp_token_id(&mut self, lp_token_id: TokenIdentifier<M>);
    fn get_lp_token_id(&self) -> &TokenIdentifier<M>;

    fn set_first_token_id(&mut self, token_id: TokenIdentifier<M>);
    fn get_first_token_id(&self) -> &TokenIdentifier<M>;

    fn set_second_token_id(&mut self, token_id: TokenIdentifier<M>);
    fn get_second_token_id(&self) -> &TokenIdentifier<M>;

    fn get_tx_input_args(&self) -> &dyn TxInputArgs<M>;
}

pub trait TxInputArgs<M: ManagedTypeApi> {
    fn are_valid(&self) -> bool;
}

pub struct AddLiquidityArgs<M: ManagedTypeApi> {
    pub first_token_amount_min: BigUint<M>,
    pub second_token_amount_min: BigUint<M>,
}

impl<M: ManagedTypeApi> AddLiquidityArgs<M> {
    pub fn new(first_token_amount_min: BigUint<M>, second_token_amount_min: BigUint<M>) -> Self {
        AddLiquidityArgs {
            first_token_amount_min,
            second_token_amount_min,
        }
    }
}

pub struct AddLiquidityContext<M: ManagedTypeApi> {
    pub tx_input_args: AddLiquidityArgs<M>,
    pub tx_input_first_payment: Option<EsdtTokenPayment<M>>,
    pub tx_input_second_payment: Option<EsdtTokenPayment<M>>,
    pub opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
    pub contract_state: State,
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> AddLiquidityContext<M> {
    pub fn new(
        tx_input_args: AddLiquidityArgs<M>,
        tx_input_first_payment: Option<EsdtTokenPayment<M>>,
        tx_input_second_payment: Option<EsdtTokenPayment<M>>,
        opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
    ) -> Self {
        AddLiquidityContext {
            tx_input_args,
            tx_input_first_payment,
            tx_input_second_payment,
            opt_accept_funds_func,
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

    fn get_tx_input_args(&self) -> &dyn TxInputArgs<M> {
        &self.tx_input_args
    }
}

impl<M: ManagedTypeApi> AddLiquidityContext<M> {
    pub fn get_tx_input_first_payment(&self) -> &Option<EsdtTokenPayment<M>> {
        &self.tx_input_first_payment
    }

    pub fn get_tx_input_second_payment(&self) -> &Option<EsdtTokenPayment<M>> {
        &self.tx_input_second_payment
    }

    pub fn is_tx_input_first_payment_valid(&self) -> bool {
        match self.tx_input_first_payment.as_ref() {
            Some(payment) => {
                payment.token_identifier == self.first_token_id
                    && payment.token_nonce == 0
                    && payment.amount >= self.tx_input_args.first_token_amount_min
            }
            None => false,
        }
    }

    pub fn is_tx_input_second_payment_valid(&self) -> bool {
        match self.tx_input_second_payment.as_ref() {
            Some(payment) => {
                payment.token_identifier == self.second_token_id
                    && payment.token_nonce == 0
                    && payment.amount >= self.tx_input_args.second_token_amount_min
            }
            None => false,
        }
    }
}

impl<M: ManagedTypeApi> TxInputArgs<M> for AddLiquidityArgs<M> {
    fn are_valid(&self) -> bool {
        self.first_token_amount_min != 0 && self.second_token_amount_min != 0
    }
}
