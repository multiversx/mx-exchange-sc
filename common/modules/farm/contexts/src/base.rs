elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use config::State;
use farm_token::FarmToken;

pub trait Context<M: ManagedTypeApi> {
    fn set_contract_state(&mut self, contract_state: State);
    fn get_contract_state(&self) -> &State;

    fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>);
    fn get_farm_token_id(&self) -> &TokenIdentifier<M>;

    fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>);
    fn get_farming_token_id(&self) -> &TokenIdentifier<M>;

    fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>);
    fn get_reward_token_id(&self) -> &TokenIdentifier<M>;

    fn set_block_nonce(&mut self, nonce: u64);
    fn get_block_nonce(&self) -> u64;

    fn set_block_epoch(&mut self, nonce: u64);
    fn get_block_epoch(&self) -> u64;

    fn set_reward_per_share(&mut self, rps: BigUint<M>);
    fn get_reward_per_share(&self) -> &BigUint<M>;

    fn set_farm_token_supply(&mut self, supply: BigUint<M>);
    fn get_farm_token_supply(&self) -> &BigUint<M>;

    fn set_division_safety_constant(&mut self, dsc: BigUint<M>);
    fn get_division_safety_constant(&self) -> &BigUint<M>;

    fn set_reward_reserve(&mut self, reward_reserve: BigUint<M>);
    fn get_reward_reserve(&self) -> &BigUint<M>;

    fn increase_reward_reserve(&mut self, amount: &BigUint<M>);
    fn decrease_reward_reserve(&mut self);

    fn update_reward_per_share(&mut self, reward_added: &BigUint<M>);

    fn set_input_attributes(&mut self, attrs: FarmTokenAttributes<M>);
    fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>>;

    fn set_initial_farming_amount(&mut self, amount: BigUint<M>);
    fn get_initial_farming_amount(&self) -> Option<&BigUint<M>>;

    fn set_position_reward(&mut self, amount: BigUint<M>);
    fn get_position_reward(&self) -> Option<&BigUint<M>>;

    fn get_storage_cache(&self) -> &StorageCache<M>;

    fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>);
    fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>>;

    fn was_output_created_with_merge(&self) -> bool;
    fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>>;
    fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool);

    fn get_caller(&self) -> &ManagedAddress<M>;

    fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>);
    fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>>;
    fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>>;

    fn get_tx_input(&self) -> &dyn TxInput<M>;
    fn is_accepted_payment(&self) -> bool;
}

pub trait TxInput<M: ManagedTypeApi> {
    fn get_args(&self) -> &dyn TxInputArgs<M>;
    fn get_payments(&self) -> &dyn TxInputPayments<M>;
}

pub trait TxInputArgs<M: ManagedTypeApi> {}

pub trait TxInputPayments<M: ManagedTypeApi> {
    fn get_first(&self) -> &EsdtTokenPayment<M>;
    fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>>;
}

pub struct StorageCache<M: ManagedTypeApi> {
    pub contract_state: State,
    pub farm_token_id: TokenIdentifier<M>,
    pub farming_token_id: TokenIdentifier<M>,
    pub reward_token_id: TokenIdentifier<M>,
    pub reward_reserve: BigUint<M>,
    pub reward_per_share: BigUint<M>,
    pub farm_token_supply: BigUint<M>,
    pub division_safety_constant: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for StorageCache<M> {
    fn default() -> Self {
        StorageCache {
            contract_state: State::Inactive,
            farm_token_id: TokenIdentifier::egld(),
            farming_token_id: TokenIdentifier::egld(),
            reward_token_id: TokenIdentifier::egld(),
            reward_reserve: BigUint::zero(),
            reward_per_share: BigUint::zero(),
            farm_token_supply: BigUint::zero(),
            division_safety_constant: BigUint::zero(),
        }
    }
}
