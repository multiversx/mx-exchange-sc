#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod custom_rewards;
pub mod farm_token_merge;

use common_errors::*;

use common_structs::{Epoch, FarmTokenAttributes};
use contexts::{
    claim_rewards_context::{ClaimRewardsContext, CompoundRewardsContext},
    enter_farm_context::EnterFarmContext,
    exit_farm_context::ExitFarmContext,
    generic::GenericContext,
    storage_cache::StorageCache,
};
use farm_token::FarmToken;

use config::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT, MAX_PERCENT,
};
use pausable::State;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + events::EventsModule
    + contexts::ctx_helper::CtxHelper
    + migration_from_v1_2::MigrationModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        mut admins: MultiValueEncoded<ManagedAddress>,
    ) {
        require!(
            reward_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(
            farming_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(division_safety_constant != 0u64, ERROR_ZERO_AMOUNT);

        let farm_token = self.farm_token().get_token_id();
        require!(reward_token_id != farm_token, ERROR_SAME_TOKEN_IDS);
        require!(farming_token_id != farm_token, ERROR_SAME_TOKEN_IDS);

        self.state().set(State::Inactive);
        self.penalty_percent().set_if_empty(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);
        self.pair_contract_address().set(&pair_contract_address);

        let caller = self.blockchain().get_caller();
        self.pause_whitelist().add(&caller);

        if admins.is_empty() {
            admins.push(caller);
        }
        self.add_admins(admins);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(&self) -> EnterFarmResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let mut storage_cache = StorageCache::new(self);
        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        let block_epoch = self.blockchain().get_block_epoch();
        let first_payment_amount = enter_farm_context.farming_token_payment.amount.clone();
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: storage_cache.reward_per_share.clone(),
            entering_epoch: block_epoch,
            original_entering_epoch: block_epoch,
            initial_farming_amount: first_payment_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: first_payment_amount.clone(),
        };

        let virtual_position_token_amount =
            EsdtTokenPayment::new(storage_cache.farm_token_id.clone(), 0, first_payment_amount);
        let virtual_position = FarmToken {
            payment: virtual_position_token_amount,
            attributes: virtual_position_attributes,
        };

        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &virtual_position,
            &enter_farm_context.additional_farm_tokens,
        );

        let caller = self.blockchain().get_caller();
        let output_farm_token_payment = new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        self.emit_enter_farm_event(
            enter_farm_context.farming_token_payment,
            new_farm_token,
            created_with_merge,
            storage_cache,
        );

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(&self) -> ExitFarmResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
        let mut storage_cache = StorageCache::new(self);
        let exit_farm_context =
            ExitFarmContext::new(payment, &storage_cache.farm_token_id, self.blockchain());

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_amount = &exit_farm_context.farm_token_payment.amount;
        let attributes = &exit_farm_context.farm_token_attributes;
        let mut reward = self.calculate_reward(
            farm_token_amount,
            &attributes.reward_per_share,
            &storage_cache.reward_per_share,
            &storage_cache.division_safety_constant,
        );
        storage_cache.reward_reserve -= &reward;

        let compounded_rewards =
            self.calculate_previously_compounded_rewards(farm_token_amount, attributes);
        reward += compounded_rewards;

        let mut initial_farming_amount =
            self.calculate_initial_farming_amount(farm_token_amount, attributes);

        if self.should_apply_penalty(attributes.entering_epoch) {
            self.burn_penalty(
                &mut initial_farming_amount,
                &storage_cache.farming_token_id,
                &storage_cache.reward_token_id,
            );
        }

        self.burn_position(&exit_farm_context.farm_token_payment);

        let farming_token_payment = EsdtTokenPayment::new(
            storage_cache.farming_token_id.clone(),
            0,
            initial_farming_amount,
        );
        let reward_payment =
            EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &farming_token_payment);
        self.send_payment_non_zero(&caller, &reward_payment);

        self.emit_exit_farm_event(
            exit_farm_context,
            farming_token_payment.clone(),
            reward_payment.clone(),
            storage_cache,
        );

        (farming_token_payment, reward_payment).into()
    }

    // #[payable("*")]
    // #[endpoint(claimRewards)]
    // fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
    //     let payments = self.call_value().all_esdt_transfers();
    //     let mut storage_cache = StorageCache::new(self);
    //     let claim_rewards_context =
    //         ClaimRewardsContext::new(payments, &storage_cache.farm_token_id);

    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

    //     self.generate_aggregated_rewards(context.get_storage_cache_mut());
    //     self.calculate_reward(&mut context);
    //     context.decrease_reward_reserve();

    //     self.calculate_initial_farming_amount(&mut context);
    //     let new_compound_reward_amount = self.calculate_new_compound_reward_amount(&context);

    //     let tx_input = context.get_tx_input();
    //     let virtual_position_token_amount = EsdtTokenPayment::new(
    //         context.get_farm_token_id().clone(),
    //         0,
    //         tx_input.first_payment.amount.clone(),
    //     );
    //     let virtual_position_attributes = FarmTokenAttributes {
    //         reward_per_share: context.get_reward_per_share().clone(),
    //         entering_epoch: context.get_input_attributes().entering_epoch,
    //         original_entering_epoch: context.get_input_attributes().original_entering_epoch,
    //         initial_farming_amount: context.get_initial_farming_amount().clone(),
    //         compounded_reward: new_compound_reward_amount,
    //         current_farm_amount: tx_input.first_payment.amount.clone(),
    //     };
    //     let virtual_position = FarmToken {
    //         payment: virtual_position_token_amount,
    //         attributes: virtual_position_attributes,
    //     };

    //     let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
    //         &virtual_position,
    //         &tx_input.additional_payments,
    //         context.get_storage_cache(),
    //     );
    //     context.set_output_position(new_farm_token, created_with_merge);

    //     self.burn_position(&context);
    //     self.commit_changes(&context);

    //     self.send_rewards(&mut context);
    //     self.execute_output_payments(&context);
    //     self.emit_claim_rewards_event(&context);

    //     self.construct_and_get_result(&context)
    // }

    // #[payable("*")]
    // #[endpoint(compoundRewards)]
    // fn compound_rewards(&self) -> CompoundRewardsResultType<Self::Api> {
    //     let payments = self.call_value().all_esdt_transfers();
    //     let mut storage_cache = StorageCache::new(self);
    //     let claim_rewards_context =
    //         CompoundRewardsContext::new(payments, &storage_cache.farm_token_id);

    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
    //     require!(
    //         context.get_farming_token_id() == context.get_reward_token_id(),
    //         ERROR_DIFFERENT_TOKEN_IDS
    //     );

    //     self.generate_aggregated_rewards(context.get_storage_cache_mut());
    //     self.calculate_reward(&mut context);
    //     context.decrease_reward_reserve();
    //     self.calculate_initial_farming_amount(&mut context);

    //     let tx_input = context.get_tx_input();
    //     let virtual_position_amount =
    //         &tx_input.first_payment.amount + context.get_position_reward();
    //     let virtual_position_token_amount = EsdtTokenPayment::new(
    //         context.get_farm_token_id().clone(),
    //         0,
    //         virtual_position_amount,
    //     );

    //     let virtual_position_compounded_reward =
    //         self.calculate_new_compound_reward_amount(&context) + context.get_position_reward();
    //     let virtual_position_current_farm_amount =
    //         &tx_input.first_payment.amount + context.get_position_reward();
    //     let virtual_position_attributes = FarmTokenAttributes {
    //         reward_per_share: context.get_reward_per_share().clone(),
    //         entering_epoch: context.get_block_epoch(),
    //         original_entering_epoch: context.get_block_epoch(),
    //         initial_farming_amount: context.get_initial_farming_amount().clone(),
    //         compounded_reward: virtual_position_compounded_reward,
    //         current_farm_amount: virtual_position_current_farm_amount,
    //     };

    //     let virtual_position = FarmToken {
    //         payment: virtual_position_token_amount,
    //         attributes: virtual_position_attributes,
    //     };

    //     let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
    //         &virtual_position,
    //         &tx_input.additional_payments,
    //         context.get_storage_cache(),
    //     );
    //     context.set_output_position(new_farm_token, created_with_merge);

    //     self.burn_position(&context);
    //     self.commit_changes(&context);

    //     self.execute_output_payments(&context);

    //     context.set_final_reward_for_emit_compound_event();
    //     self.emit_compound_rewards_event(&context);

    //     context.get_output_payments().get(0)
    // }

    fn validate_contract_state(&self, current_state: State, farm_token_id: &TokenIdentifier) {
        require!(current_state == State::Active, ERROR_NOT_ACTIVE);
        require!(
            farm_token_id.is_valid_esdt_identifier(),
            ERROR_NO_FARM_TOKEN
        );
    }

    fn burn_farming_tokens(
        &self,
        farming_amount: &BigUint,
        farming_token_id: &TokenIdentifier,
        reward_token_id: &TokenIdentifier,
    ) {
        let pair_contract_address = self.pair_contract_address().get();
        if pair_contract_address.is_zero() {
            self.send()
                .esdt_local_burn(farming_token_id, 0, farming_amount);
        } else {
            let gas_limit = self.burn_gas_limit().get();
            self.pair_contract_proxy(pair_contract_address)
                .remove_liquidity_and_burn_token(reward_token_id.clone())
                .add_esdt_token_transfer(farming_token_id.clone(), 0, farming_amount.clone())
                .with_gas_limit(gas_limit)
                .transfer_execute();
        }
    }

    fn create_farm_tokens_by_merging(
        &self,
        virtual_position: &FarmToken<Self::Api>,
        additional_positions: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> (FarmToken<Self::Api>, bool) {
        let farm_token_id = virtual_position.payment.token_identifier.clone();
        let additional_payments_len = additional_positions.len();
        let merged_attributes =
            self.get_merged_farm_token_attributes(additional_positions, Some(virtual_position));

        self.burn_farm_tokens_from_payments(additional_positions);

        let new_amount = merged_attributes.current_farm_amount.clone();
        let new_tokens = self.mint_farm_tokens(farm_token_id, new_amount, &merged_attributes);

        let new_farm_token = FarmToken {
            payment: new_tokens,
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        (new_farm_token, is_merged)
    }

    fn send_back_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        destination: &ManagedAddress,
    ) {
        self.send()
            .direct_esdt(destination, farming_token_id, 0, farming_amount);
    }

    // fn send_rewards(&self, context: &mut GenericContext<Self::Api>) {
    //     if context.get_position_reward() > &0u64 {
    //         self.send_tokens_non_zero(
    //             context.get_caller(),
    //             context.get_reward_token_id(),
    //             0,
    //             context.get_position_reward(),
    //         );
    //     }

    //     context.set_final_reward(EsdtTokenPayment::new(
    //         context.get_reward_token_id().clone(),
    //         0,
    //         context.get_position_reward().clone(),
    //     ));
    // }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        self.calculate_reward(
            &amount,
            &attributes.reward_per_share,
            &storage_cache.reward_per_share,
            &storage_cache.division_safety_constant,
        )
    }

    fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
        entering_epoch + self.minimum_farming_epochs().get() > self.blockchain().get_block_epoch()
    }

    fn get_penalty_amount(&self, amount: &BigUint) -> BigUint {
        amount * self.penalty_percent().get() / MAX_PERCENT
    }

    fn burn_penalty(
        &self,
        initial_farming_amount: &mut BigUint,
        farming_token_id: &TokenIdentifier,
        reward_token_id: &TokenIdentifier,
    ) {
        let penalty_amount = self.get_penalty_amount(initial_farming_amount);
        if penalty_amount > 0u64 {
            self.burn_farming_tokens(&penalty_amount, farming_token_id, reward_token_id);

            *initial_farming_amount -= penalty_amount;
        }
    }

    fn burn_position(&self, farm_token_payment: &EsdtTokenPayment<Self::Api>) {
        self.burn_farm_tokens(
            &farm_token_payment.token_identifier,
            farm_token_payment.token_nonce,
            &farm_token_payment.amount,
        );
    }

    fn calculate_initial_farming_amount(
        &self,
        farm_token_amount: &BigUint,
        farm_token_attributes: &FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.rule_of_three_non_zero_result(
            farm_token_amount,
            &farm_token_attributes.current_farm_amount,
            &farm_token_attributes.initial_farming_amount,
        )
    }

    fn calculate_previously_compounded_rewards(
        &self,
        farm_token_amount: &BigUint,
        farm_token_attributes: &FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.rule_of_three(
            farm_token_amount,
            &farm_token_attributes.current_farm_amount,
            &farm_token_attributes.compounded_reward,
        )
    }

    // fn calculate_new_compound_reward_amount(&self, context: &GenericContext<Self::Api>) -> BigUint {
    //     let first_payment = &context.get_tx_input().first_payment;
    //     let attributes = context.get_input_attributes();

    //     self.rule_of_three(
    //         &first_payment.amount,
    //         &attributes.current_farm_amount,
    //         &attributes.compounded_reward,
    //     )
    // }
}
