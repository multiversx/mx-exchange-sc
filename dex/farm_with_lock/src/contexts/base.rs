elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::State;

pub trait Context<M: ManagedTypeApi> {
    fn set_contract_state(&mut self, contract_state: State);
    fn get_contract_state(&self) -> &State;

    fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>);
    fn get_farm_token_id(&self) -> &TokenIdentifier<M>;

    fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>);
    fn get_farming_token_id(&self) -> &TokenIdentifier<M>;

    fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>);
    fn get_reward_token_id(&self) -> &TokenIdentifier<M>;

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
    pub farm_token_id: TokenIdentifier<M>,
    pub farming_token_id: TokenIdentifier<M>,
    pub reward_token_id: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> Default for StorageCache<M> {
    fn default() -> Self {
        StorageCache {
            contract_state: State::Inactive,
            farm_token_id: TokenIdentifier::egld(),
            farming_token_id: TokenIdentifier::egld(),
            reward_token_id: TokenIdentifier::egld(),
        }
    }
}
