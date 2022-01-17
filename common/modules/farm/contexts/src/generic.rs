elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use farm_token::FarmToken;

use config::State;

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
    args: GenericArgs<M>,
    payments: GenericPayments<M>,
    attributes: Option<FarmTokenAttributes<M>>,
}

pub struct GenericArgs<M: ManagedTypeApi> {
    opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
}

pub struct GenericPayments<M: ManagedTypeApi> {
    first_payment: EsdtTokenPayment<M>,
    additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

impl<M: ManagedTypeApi> GenericTxInput<M> {
    pub fn new(args: GenericArgs<M>, payments: GenericPayments<M>) -> Self {
        GenericTxInput {
            args,
            payments,
            attributes: None,
        }
    }
}

impl<M: ManagedTypeApi> GenericArgs<M> {
    pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
        GenericArgs {
            opt_accept_funds_func,
        }
    }
}

impl<M: ManagedTypeApi> GenericPayments<M> {
    pub fn new(
        first_payment: EsdtTokenPayment<M>,
        additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
    ) -> Self {
        GenericPayments {
            first_payment,
            additional_payments,
        }
    }
}

impl<M: ManagedTypeApi> GenericContext<M> {
    pub fn new(tx_input: GenericTxInput<M>, caller: ManagedAddress<M>) -> Self {
        GenericContext {
            caller,
            tx_input,
            block_nonce: 0,
            block_epoch: 0,
            position_reward: BigUint::zero(),
            storage_cache: StorageCache::default(),
            initial_farming_amount: BigUint::zero(),
            final_reward: None,
            output_attributes: None,
            output_created_with_merge: true,
            output_payments: ManagedVec::new(),
        }
    }
}

impl<M: ManagedTypeApi> GenericContext<M> {
    #[inline]
    pub fn set_contract_state(&mut self, contract_state: State) {
        self.storage_cache.contract_state = contract_state;
    }

    #[inline]
    pub fn get_contract_state(&self) -> &State {
        &self.storage_cache.contract_state
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
    pub fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
        &self.tx_input.args.opt_accept_funds_func
    }

    #[inline]
    pub fn get_tx_input(&self) -> &GenericTxInput<M> {
        &self.tx_input
    }

    #[inline]
    pub fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
        self.storage_cache.farm_token_id = farm_token_id
    }

    #[inline]
    pub fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farm_token_id
    }

    #[inline]
    pub fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
        self.storage_cache.farming_token_id = farming_token_id
    }

    #[inline]
    pub fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.farming_token_id
    }

    #[inline]
    pub fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
        self.storage_cache.reward_token_id = reward_token_id;
    }

    #[inline]
    pub fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
        &self.storage_cache.reward_token_id
    }

    #[inline]
    pub fn set_block_nonce(&mut self, nonce: u64) {
        self.block_nonce = nonce;
    }

    #[inline]
    pub fn get_block_nonce(&self) -> u64 {
        self.block_nonce
    }

    #[inline]
    pub fn set_block_epoch(&mut self, epoch: u64) {
        self.block_epoch = epoch;
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
    pub fn set_division_safety_constant(&mut self, dsc: BigUint<M>) {
        self.storage_cache.division_safety_constant = dsc;
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
    pub fn increase_reward_reserve(&mut self, amount: &BigUint<M>) {
        self.storage_cache.reward_reserve += amount;
    }

    #[inline]
    pub fn decrease_reward_reserve(&mut self) {
        self.storage_cache.reward_reserve -= &self.position_reward;
    }

    #[inline]
    pub fn update_reward_per_share(&mut self, reward_added: &BigUint<M>) {
        if self.storage_cache.farm_token_supply != 0u64 {
            self.storage_cache.reward_per_share += reward_added
                * &self.storage_cache.division_safety_constant
                / &self.storage_cache.farm_token_supply;
        }
    }

    #[inline]
    pub fn get_storage_cache(&self) -> &StorageCache<M> {
        &self.storage_cache
    }

    #[inline]
    pub fn set_input_attributes(&mut self, attr: FarmTokenAttributes<M>) {
        self.tx_input.attributes = Some(attr);
    }

    #[inline]
    pub fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
        self.tx_input.attributes.as_ref()
    }

    #[inline]
    pub fn set_position_reward(&mut self, amount: BigUint<M>) {
        self.position_reward = amount;
    }

    #[inline]
    pub fn get_position_reward(&self) -> Option<&BigUint<M>> {
        Some(&self.position_reward)
    }

    #[inline]
    pub fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {
        self.initial_farming_amount = amount;
    }

    #[inline]
    pub fn get_initial_farming_amount(&self) -> Option<&BigUint<M>> {
        Some(&self.initial_farming_amount)
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

    #[inline]
    pub fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
        self.output_payments.push(position.token_amount);
        self.output_created_with_merge = created_with_merge;
        self.output_attributes = Some(position.attributes);
    }

    #[inline]
    pub fn set_final_reward_for_emit_compound_event(&mut self) {
        self.final_reward = Some(EsdtTokenPayment::new(
            self.storage_cache.reward_token_id.clone(),
            0,
            self.position_reward.clone(),
        ));
    }

    #[inline]
    pub fn is_accepted_payment_enter(&self) -> bool {
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

    #[inline]
    pub fn is_accepted_payment_exit(&self) -> bool {
        let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
            == self.storage_cache.farm_token_id
            && self.tx_input.payments.first_payment.token_nonce != 0
            && self.tx_input.payments.first_payment.amount != 0u64;

        if !first_payment_pass {
            return false;
        }

        self.tx_input.payments.additional_payments.is_empty()
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
        let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
            == self.storage_cache.farm_token_id
            && self.tx_input.payments.first_payment.token_nonce != 0
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

    #[inline]
    pub fn increase_position_reward(&mut self, amount: &BigUint<M>) {
        self.position_reward += amount;
    }

    #[inline]
    pub fn decrease_farming_token_amount(&mut self, amount: &BigUint<M>) {
        self.initial_farming_amount -= amount;
    }
}

impl<M: ManagedTypeApi> GenericPayments<M> {
    #[inline]
    pub fn get_first(&self) -> &EsdtTokenPayment<M> {
        &self.first_payment
    }

    #[inline]
    pub fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>> {
        Some(&self.additional_payments)
    }
}

impl<M: ManagedTypeApi> GenericTxInput<M> {
    #[inline]
    pub fn get_args(&self) -> &GenericArgs<M> {
        &self.args
    }

    #[inline]
    pub fn get_payments(&self) -> &GenericPayments<M> {
        &self.payments
    }
}
