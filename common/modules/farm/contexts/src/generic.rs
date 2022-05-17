elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use elrond_wasm::api::{CallTypeApi, StorageMapperApi};
use farm_token::FarmToken;

use config::State;

pub trait FarmContracTraitBounds =
    config::ConfigModule
        + token_send::TokenSendModule
        + rewards::RewardsModule
        + farm_token::FarmTokenModule
        + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule;

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

impl<M: ManagedTypeApi + StorageMapperApi + CallTypeApi> StorageCache<M> {
    pub fn new<C: FarmContracTraitBounds<Api = M>>(farm_sc: &C) -> Self {
        StorageCache {
            contract_state: farm_sc.state().get(),
            farm_token_id: farm_sc.farm_token().get_token_id(),
            farming_token_id: farm_sc.farming_token_id().get(),
            reward_token_id: farm_sc.reward_token_id().get(),
            reward_reserve: farm_sc.reward_reserve().get(),
            reward_per_share: farm_sc.reward_per_share().get(),
            farm_token_supply: farm_sc.farm_token_supply().get(),
            division_safety_constant: farm_sc.division_safety_constant().get(),
        }
    }
}

pub struct GenericContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: GenericTxInput<M>,
    block_nonce: u64,
    block_epoch: u64,
    position_reward: BigUint<M>,
    storage_cache: StorageCache<M>,
    initial_farming_amount: BigUint<M>,
    final_reward: Option<EsdtTokenPayment<M>>,
    output_attributes: Option<FarmTokenAttributes<M>>,
    output_created_with_merge: bool,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct GenericTxInput<M: ManagedTypeApi> {
    pub first_payment: EsdtTokenPayment<M>,
    pub additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
    attributes: Option<FarmTokenAttributes<M>>,
}

impl<M: ManagedTypeApi + StorageMapperApi + CallTypeApi + CallValueApi> GenericTxInput<M> {
    pub fn new<C: FarmContracTraitBounds<Api = M>>(farm_sc: &C) -> Self {
        let mut payments = farm_sc.call_value().all_esdt_transfers();

        let first_payment = payments.get(0);
        payments.remove(0);

        GenericTxInput {
            first_payment,
            additional_payments: payments,
            attributes: None,
        }
    }
}

impl<M: ManagedTypeApi + BlockchainApi + StorageMapperApi + CallTypeApi + CallValueApi>
    GenericContext<M>
{
    pub fn new<C: FarmContracTraitBounds<Api = M>>(farm_sc: &C) -> Self {
        let storage_cache = StorageCache::new(farm_sc);
        let mut tx_input = GenericTxInput::new(farm_sc);

        if tx_input.first_payment.token_identifier == storage_cache.farm_token_id {
            let attributes: FarmTokenAttributes<M> = farm_sc.get_farm_token_attributes(
                &tx_input.first_payment.token_identifier,
                tx_input.first_payment.token_nonce,
            );
            tx_input.attributes = Some(attributes);
        }

        GenericContext {
            caller: farm_sc.blockchain().get_caller(),
            block_epoch: farm_sc.blockchain().get_block_epoch(),
            block_nonce: farm_sc.blockchain().get_block_nonce(),
            tx_input,
            storage_cache,
            position_reward: BigUint::zero(),
            initial_farming_amount: BigUint::zero(),
            final_reward: None,
            output_attributes: None,
            output_created_with_merge: true,
            output_payments: ManagedVec::new(),
        }
    }

    #[inline]
    pub fn set_contract_state(&mut self, contract_state: State) {
        self.storage_cache.contract_state = contract_state;
    }

    #[inline]
    pub fn get_contract_state(&self) -> State {
        self.storage_cache.contract_state
    }

    #[inline]
    pub fn get_caller(&self) -> &ManagedAddress<M> {
        &self.caller
    }

    #[inline]
    pub fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
        self.output_payments = payments
    }

    #[inline]
    pub fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
        &self.output_payments
    }

    #[inline]
    pub fn get_tx_input(&self) -> &GenericTxInput<M> {
        &self.tx_input
    }

    #[inline]
    pub fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farm_token_id
    }

    #[inline]
    pub fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farming_token_id
    }

    #[inline]
    pub fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.reward_token_id
    }

    #[inline]
    pub fn get_block_nonce(&self) -> u64 {
        self.block_nonce
    }

    #[inline]
    pub fn get_block_epoch(&self) -> u64 {
        self.block_epoch
    }

    #[inline]
    pub fn set_reward_per_share(&mut self, rps: BigUint<M>) {
        self.storage_cache.reward_per_share = rps;
    }

    #[inline]
    pub fn get_reward_per_share(&self) -> &BigUint<M> {
        &self.storage_cache.reward_per_share
    }

    #[inline]
    pub fn set_farm_token_supply(&mut self, supply: BigUint<M>) {
        self.storage_cache.farm_token_supply = supply;
    }

    #[inline]
    pub fn get_farm_token_supply(&self) -> &BigUint<M> {
        &self.storage_cache.farm_token_supply
    }

    #[inline]
    pub fn get_division_safety_constant(&self) -> &BigUint<M> {
        &self.storage_cache.division_safety_constant
    }

    #[inline]
    pub fn set_reward_reserve(&mut self, rr: BigUint<M>) {
        self.storage_cache.reward_reserve = rr;
    }

    #[inline]
    pub fn get_reward_reserve(&self) -> &BigUint<M> {
        &self.storage_cache.reward_reserve
    }

    #[inline]
    pub fn decrease_reward_reserve(&mut self) {
        self.storage_cache.reward_reserve -= &self.position_reward;
    }

    #[inline]
    pub fn get_storage_cache(&self) -> &StorageCache<M> {
        &self.storage_cache
    }

    #[inline]
    pub fn get_storage_cache_mut(&mut self) -> &mut StorageCache<M> {
        &mut self.storage_cache
    }

    #[inline]
    pub fn get_input_attributes(&self) -> &FarmTokenAttributes<M> {
        if let Some(attr) = &self.tx_input.attributes {
            return attr;
        } else {
            M::error_api_impl().signal_error(b"No farm token attributes");
        }
    }

    #[inline]
    pub fn set_position_reward(&mut self, amount: BigUint<M>) {
        self.position_reward = amount;
    }

    #[inline]
    pub fn get_position_reward(&self) -> &BigUint<M> {
        &self.position_reward
    }

    #[inline]
    pub fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {
        self.initial_farming_amount = amount;
    }

    #[inline]
    pub fn get_initial_farming_amount(&self) -> &BigUint<M> {
        &self.initial_farming_amount
    }

    #[inline]
    pub fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>) {
        self.final_reward = Some(payment);
    }

    #[inline]
    pub fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>> {
        self.final_reward.as_ref()
    }

    #[inline]
    pub fn was_output_created_with_merge(&self) -> bool {
        self.output_created_with_merge
    }

    #[inline]
    pub fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
        self.output_attributes.as_ref()
    }

    pub fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
        self.output_payments.push(position.payment);
        self.output_created_with_merge = created_with_merge;
        self.output_attributes = Some(position.attributes);
    }

    pub fn set_final_reward_for_emit_compound_event(&mut self) {
        self.final_reward = Some(EsdtTokenPayment::new(
            self.storage_cache.reward_token_id.clone(),
            0,
            self.position_reward.clone(),
        ));
    }

    pub fn is_accepted_payment_enter(&self) -> bool {
        let first_payment_pass = self.tx_input.first_payment.token_identifier
            == self.storage_cache.farming_token_id
            && self.tx_input.first_payment.token_nonce == 0
            && self.tx_input.first_payment.amount != 0u64;

        if !first_payment_pass {
            return false;
        }

        for payment in self.tx_input.additional_payments.iter() {
            let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                && payment.token_nonce != 0
                && payment.amount != 0;

            if !payment_pass {
                return false;
            }
        }

        true
    }

    pub fn is_accepted_payment_exit(&self) -> bool {
        let first_payment_pass = self.tx_input.first_payment.token_identifier
            == self.storage_cache.farm_token_id
            && self.tx_input.first_payment.token_nonce != 0
            && self.tx_input.first_payment.amount != 0u64;

        if !first_payment_pass {
            return false;
        }

        self.tx_input.additional_payments.is_empty()
    }

    #[inline]
    pub fn is_accepted_payment_claim(&self) -> bool {
        self.is_accepted_payment_claim_compound()
    }

    #[inline]
    pub fn is_accepted_payment_compound(&self) -> bool {
        self.is_accepted_payment_claim_compound()
    }

    fn is_accepted_payment_claim_compound(&self) -> bool {
        let first_payment_pass = self.tx_input.first_payment.token_identifier
            == self.storage_cache.farm_token_id
            && self.tx_input.first_payment.token_nonce != 0
            && self.tx_input.first_payment.amount != 0u64;

        if !first_payment_pass {
            return false;
        }

        for payment in self.tx_input.additional_payments.iter() {
            let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                && payment.token_nonce != 0
                && payment.amount != 0;

            if !payment_pass {
                return false;
            }
        }

        true
    }

    #[inline]
    pub fn increase_position_reward(&mut self, amount: &BigUint<M>) {
        self.position_reward += amount;
    }

    #[inline]
    pub fn decrease_farming_token_amount(&mut self, amount: &BigUint<M>) {
        self.initial_farming_amount -= amount;
    }
}
