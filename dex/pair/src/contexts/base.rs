elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::State;

pub trait Context<M: ManagedTypeApi> {
    fn set_contract_state(&mut self, contract_state: State);
    fn get_contract_state(&self) -> &State;

    fn set_lp_token_id(&mut self, lp_token_id: TokenIdentifier<M>);
    fn get_lp_token_id(&self) -> &TokenIdentifier<M>;

    fn set_first_token_id(&mut self, token_id: TokenIdentifier<M>);
    fn get_first_token_id(&self) -> &TokenIdentifier<M>;

    fn set_second_token_id(&mut self, token_id: TokenIdentifier<M>);
    fn get_second_token_id(&self) -> &TokenIdentifier<M>;

    fn set_first_token_reserve(&mut self, amount: BigUint<M>);
    fn get_first_token_reserve(&self) -> &BigUint<M>;

    fn set_second_token_reserve(&mut self, amount: BigUint<M>);
    fn get_second_token_reserve(&self) -> &BigUint<M>;

    fn set_lp_token_supply(&mut self, amount: BigUint<M>);
    fn get_lp_token_supply(&self) -> &BigUint<M>;

    fn set_initial_k(&mut self, k: BigUint<M>);
    fn get_initial_k(&self) -> &BigUint<M>;

    fn get_caller(&self) -> &ManagedAddress<M>;

    fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>);
    fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>>;
    fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>>;

    fn get_tx_input(&self) -> &dyn TxInput<M>;
}

pub trait TxInput<M: ManagedTypeApi> {
    fn get_args(&self) -> &dyn TxInputArgs<M>;
    fn get_payments(&self) -> &dyn TxInputPayments<M>;

    fn is_valid(&self) -> bool;
}

pub trait TxInputArgs<M: ManagedTypeApi> {
    fn are_valid(&self) -> bool;
}

pub trait TxInputPayments<M: ManagedTypeApi> {
    fn are_valid(&self) -> bool;
}

pub struct StorageCache<M: ManagedTypeApi> {
    pub contract_state: State,
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
    pub first_token_reserve: BigUint<M>,
    pub second_token_reserve: BigUint<M>,
    pub lp_token_supply: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for StorageCache<M> {
    fn default() -> Self {
        StorageCache {
            contract_state: State::Inactive,
            lp_token_id: TokenIdentifier::egld(),
            first_token_id: TokenIdentifier::egld(),
            second_token_id: TokenIdentifier::egld(),
            first_token_reserve: BigUint::zero(),
            second_token_reserve: BigUint::zero(),
            lp_token_supply: BigUint::zero(),
        }
    }
}
