#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod base_farm_validation;
pub mod claim_rewards;
pub mod compound_rewards;
pub mod enter_farm;
pub mod exit_farm;
pub mod partial_positions;

use claim_rewards::InternalClaimRewardsResult;
use common_errors::*;
use common_structs::{FarmTokenAttributes, PaymentsVec};
use compound_rewards::InternalCompoundRewardsResult;
use contexts::storage_cache::StorageCache;
use enter_farm::InternalEnterFarmResult;
use exit_farm::InternalExitFarmResult;
use pausable::State;

pub type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::module]
pub trait FarmBaseImpl:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + token_merge_helper::TokenMergeHelperModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_farm_validation::BaseFarmValidationModule
    + partial_positions::PartialPositionsModule
    + enter_farm::BaseEnterFarmModule
    + claim_rewards::BaseClaimRewardsModule
    + compound_rewards::BaseCompoundRewardsModule
    + exit_farm::BaseExitFarmModule
{
    // use only in SCs importing
    // #[proxy]
    // fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    fn base_farm_init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
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
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);

        let caller = self.blockchain().get_caller();
        self.pause_whitelist().add(&caller);

        if admins.is_empty() {
            admins.push(caller);
        }
        self.add_admins(admins);
    }

    fn default_enter_farm_impl(
        &self,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, FarmTokenAttributes<Self::Api>> {
        self.enter_farm_base(
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_create_enter_farm_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    fn default_claim_rewards_impl(
        &self,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, FarmTokenAttributes<Self::Api>> {
        self.claim_rewards_base(
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
            Self::default_create_claim_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    // TODO: Think about reusing some of the logic from claim_rewards
    // How to fix: Don't mint tokens, and allow caller to do what they wish with token/attributes
    fn default_compound_rewards_impl(
        &self,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self, FarmTokenAttributes<Self::Api>> {
        self.compound_rewards_base(
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
            Self::default_create_compound_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    fn default_exit_farm_impl(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> InternalExitFarmResult<Self, FarmTokenAttributes<Self::Api>> {
        self.exit_farm_base(
            payment,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
        )
    }

    // #[payable("*")]
    // #[endpoint(exitFarm)]
    // fn exit_farm(&self) -> ExitFarmResultType<Self::Api> {
    //     let payment = self.call_value().single_esdt();
    //     let mut storage_cache = StorageCache::new(self);
    //     let exit_farm_context =
    //         ExitFarmContext::new(payment, &storage_cache.farm_token_id, self.blockchain());

    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
    //     self.generate_aggregated_rewards(&mut storage_cache);

    //     let farm_token_amount = &exit_farm_context.farm_token.payment.amount;
    //     let attributes = &exit_farm_context.farm_token.attributes;
    //     let mut reward = self.calculate_reward(
    //         farm_token_amount,
    //         &attributes.reward_per_share,
    //         &storage_cache.reward_per_share,
    //         &storage_cache.division_safety_constant,
    //     );
    //     storage_cache.reward_reserve -= &reward;

    //     let prev_compounded_rewards =
    //         self.calculate_previously_compounded_rewards(farm_token_amount, attributes);
    //     reward += prev_compounded_rewards;

    //     let mut initial_farming_amount =
    //         self.calculate_initial_farming_amount(farm_token_amount, attributes);

    //     if self.should_apply_penalty(attributes.entering_epoch) {
    //         self.burn_penalty(
    //             &mut initial_farming_amount,
    //             &storage_cache.farming_token_id,
    //             &storage_cache.reward_token_id,
    //         );
    //     }

    //     self.burn_position(&exit_farm_context.farm_token.payment);

    //     let farming_token_payment = EsdtTokenPayment::new(
    //         storage_cache.farming_token_id.clone(),
    //         0,
    //         initial_farming_amount,
    //     );
    //     let reward_payment =
    //         EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

    //     let caller = self.blockchain().get_caller();
    //     self.send_payment_non_zero(&caller, &farming_token_payment);
    //     self.send_payment_non_zero(&caller, &reward_payment);

    //     self.emit_exit_farm_event(
    //         exit_farm_context,
    //         farming_token_payment.clone(),
    //         reward_payment.clone(),
    //         storage_cache,
    //     );

    //     (farming_token_payment, reward_payment).into()
    // }

    // #[payable("*")]
    // #[endpoint(claimRewards)]
    // fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
    //     let payments = self.call_value().all_esdt_transfers();
    //     let mut storage_cache = StorageCache::new(self);
    //     let claim_rewards_context =
    //         ClaimRewardsContext::new(payments, &storage_cache.farm_token_id, self.blockchain());

    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
    //     self.generate_aggregated_rewards(&mut storage_cache);

    //     let farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
    //     let attributes = &claim_rewards_context.first_farm_token.attributes;
    //     let reward = self.calculate_reward(
    //         farm_token_amount,
    //         &attributes.reward_per_share,
    //         &storage_cache.reward_per_share,
    //         &storage_cache.division_safety_constant,
    //     );
    //     storage_cache.reward_reserve -= &reward;

    //     let initial_farming_amount =
    //         self.calculate_initial_farming_amount(farm_token_amount, attributes);
    //     let new_compound_reward_amount =
    //         self.calculate_new_compound_reward_amount(farm_token_amount, attributes);

    //     let virtual_position_token_amount = EsdtTokenPayment::new(
    //         storage_cache.farm_token_id.clone(),
    //         0,
    //         farm_token_amount.clone(),
    //     );
    //     let virtual_position_attributes = FarmTokenAttributes {
    //         reward_per_share: storage_cache.reward_per_share.clone(),
    //         entering_epoch: attributes.entering_epoch,
    //         original_entering_epoch: attributes.original_entering_epoch,
    //         initial_farming_amount,
    //         compounded_reward: new_compound_reward_amount,
    //         current_farm_amount: farm_token_amount.clone(),
    //     };

    //     let virtual_position = FarmToken {
    //         payment: virtual_position_token_amount,
    //         attributes: virtual_position_attributes,
    //     };
    //     let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
    //         virtual_position,
    //         &claim_rewards_context.additional_payments,
    //     );

    //     self.burn_position(&claim_rewards_context.first_farm_token.payment);

    //     let new_farm_token_payment = new_farm_token.payment.clone();
    //     let reward_payment =
    //         EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

    //     let caller = self.blockchain().get_caller();
    //     self.send_payment_non_zero(&caller, &new_farm_token_payment);
    //     self.send_payment_non_zero(&caller, &reward_payment);

    //     self.emit_claim_rewards_event(
    //         claim_rewards_context,
    //         new_farm_token,
    //         created_with_merge,
    //         reward_payment.clone(),
    //         storage_cache,
    //     );

    //     (new_farm_token_payment, reward_payment).into()
    // }

    // #[payable("*")]
    // #[endpoint(compoundRewards)]
    // fn compound_rewards(&self) -> CompoundRewardsResultType<Self::Api> {
    //     let payments = self.call_value().all_esdt_transfers();
    //     let mut storage_cache = StorageCache::new(self);
    //     let compound_rewards_context =
    //         CompoundRewardsContext::new(payments, &storage_cache.farm_token_id, self.blockchain());

    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
    //     require!(
    //         storage_cache.farming_token_id == storage_cache.reward_token_id,
    //         ERROR_DIFFERENT_TOKEN_IDS
    //     );

    //     self.generate_aggregated_rewards(&mut storage_cache);

    //     let farm_token_amount = &compound_rewards_context.first_farm_token.payment.amount;
    //     let attributes = &compound_rewards_context.first_farm_token.attributes;
    //     let reward = self.calculate_reward(
    //         farm_token_amount,
    //         &attributes.reward_per_share,
    //         &storage_cache.reward_per_share,
    //         &storage_cache.division_safety_constant,
    //     );
    //     storage_cache.reward_reserve -= &reward;

    //     let initial_farming_amount =
    //         self.calculate_initial_farming_amount(farm_token_amount, attributes);
    //     let new_compound_reward_amount =
    //         self.calculate_new_compound_reward_amount(farm_token_amount, attributes);

    //     let virtual_position_amount = farm_token_amount + &reward;
    //     let virtual_position_token_amount = EsdtTokenPayment::new(
    //         storage_cache.farm_token_id.clone(),
    //         0,
    //         virtual_position_amount,
    //     );

    //     let block_epoch = self.blockchain().get_block_epoch();
    //     let virtual_position_compounded_reward = &new_compound_reward_amount + &reward;
    //     let virtual_position_current_farm_amount = farm_token_amount + &reward;
    //     let virtual_position_attributes = FarmTokenAttributes {
    //         reward_per_share: storage_cache.reward_per_share.clone(),
    //         entering_epoch: block_epoch,
    //         original_entering_epoch: block_epoch,
    //         initial_farming_amount,
    //         compounded_reward: virtual_position_compounded_reward,
    //         current_farm_amount: virtual_position_current_farm_amount,
    //     };

    //     let virtual_position = FarmToken {
    //         payment: virtual_position_token_amount,
    //         attributes: virtual_position_attributes,
    //     };
    //     let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
    //         virtual_position,
    //         &compound_rewards_context.additional_payments,
    //     );

    //     self.burn_position(&compound_rewards_context.first_farm_token.payment);

    //     let new_farm_token_payment = new_farm_token.payment.clone();
    //     let caller = self.blockchain().get_caller();
    //     self.send_payment_non_zero(&caller, &new_farm_token_payment);

    //     self.emit_compound_rewards_event(
    //         compound_rewards_context,
    //         new_farm_token,
    //         created_with_merge,
    //         reward,
    //         storage_cache,
    //     );

    //     new_farm_token_payment
    // }

    // #[payable("*")]
    // #[endpoint(mergeFarmTokens)]
    // fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
    //     let payments = self.call_value().all_esdt_transfers();

    //     let attrs = self.get_merged_farm_token_attributes(&payments, Option::None);
    //     let farm_token_id = self.farm_token().get_token_id();
    //     self.burn_farm_tokens_from_payments(&payments);

    //     let new_tokens =
    //         self.mint_farm_tokens(farm_token_id, attrs.current_farm_amount.clone(), &attrs);

    //     let caller = self.blockchain().get_caller();
    //     self.send_payment_non_zero(&caller, &new_tokens);

    //     new_tokens
    // }

    // fn validate_contract_state(&self, current_state: State, farm_token_id: &TokenIdentifier) {
    //     require!(current_state == State::Active, ERROR_NOT_ACTIVE);
    //     require!(
    //         farm_token_id.is_valid_esdt_identifier(),
    //         ERROR_NO_FARM_TOKEN
    //     );
    // }

    // fn burn_farming_tokens(
    //     &self,
    //     farming_amount: &BigUint,
    //     farming_token_id: &TokenIdentifier,
    //     reward_token_id: &TokenIdentifier,
    // ) {
    //     let pair_contract_address = self.pair_contract_address().get();
    //     if pair_contract_address.is_zero() {
    //         self.send()
    //             .esdt_local_burn(farming_token_id, 0, farming_amount);
    //     } else {
    //         let gas_limit = self.burn_gas_limit().get();
    //         self.pair_contract_proxy(pair_contract_address)
    //             .remove_liquidity_and_burn_token(reward_token_id.clone())
    //             .add_esdt_token_transfer(farming_token_id.clone(), 0, farming_amount.clone())
    //             .with_gas_limit(gas_limit)
    //             .transfer_execute();
    //     }
    // }

    // fn create_farm_tokens_by_merging(
    //     &self,
    //     virtual_position: FarmToken<Self::Api>,
    //     additional_positions: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    // ) -> (FarmToken<Self::Api>, bool) {
    //     let farm_token_id = virtual_position.payment.token_identifier.clone();
    //     let additional_payments_len = additional_positions.len();
    //     let merged_attributes =
    //         self.get_merged_farm_token_attributes(additional_positions, Some(virtual_position));

    //     self.burn_farm_tokens_from_payments(additional_positions);

    //     let new_amount = merged_attributes.current_farm_amount.clone();
    //     let new_tokens = self.mint_farm_tokens(farm_token_id, new_amount, &merged_attributes);

    //     let new_farm_token = FarmToken {
    //         payment: new_tokens,
    //         attributes: merged_attributes,
    //     };
    //     let is_merged = additional_payments_len != 0;

    //     (new_farm_token, is_merged)
    // }

    // fn send_back_farming_tokens(
    //     &self,
    //     farming_token_id: &TokenIdentifier,
    //     farming_amount: &BigUint,
    //     destination: &ManagedAddress,
    // ) {
    //     self.send()
    //         .direct_esdt(destination, farming_token_id, 0, farming_amount);
    // }

    // #[view(calculateRewardsForGivenPosition)]
    // fn calculate_rewards_for_given_position(
    //     &self,
    //     amount: BigUint,
    //     attributes: FarmTokenAttributes<Self::Api>,
    // ) -> BigUint {
    //     let mut storage_cache = StorageCache::new(self);
    //     self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
    //     self.generate_aggregated_rewards(&mut storage_cache);

    //     self.calculate_reward(
    //         &amount,
    //         &attributes.reward_per_share,
    //         &storage_cache.reward_per_share,
    //         &storage_cache.division_safety_constant,
    //     )
    // }

    // fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
    //     entering_epoch + self.minimum_farming_epochs().get() > self.blockchain().get_block_epoch()
    // }

    // fn get_penalty_amount(&self, amount: &BigUint) -> BigUint {
    //     amount * self.penalty_percent().get() / MAX_PERCENT
    // }

    // fn burn_penalty(
    //     &self,
    //     initial_farming_amount: &mut BigUint,
    //     farming_token_id: &TokenIdentifier,
    //     reward_token_id: &TokenIdentifier,
    // ) {
    //     let penalty_amount = self.get_penalty_amount(initial_farming_amount);
    //     if penalty_amount > 0u64 {
    //         self.burn_farming_tokens(&penalty_amount, farming_token_id, reward_token_id);

    //         *initial_farming_amount -= penalty_amount;
    //     }
    // }

    // fn burn_position(&self, farm_token_payment: &EsdtTokenPayment<Self::Api>) {
    //     self.burn_farm_tokens(
    //         &farm_token_payment.token_identifier,
    //         farm_token_payment.token_nonce,
    //         &farm_token_payment.amount,
    //     );
    // }

    // fn calculate_initial_farming_amount(
    //     &self,
    //     farm_token_amount: &BigUint,
    //     farm_token_attributes: &FarmTokenAttributes<Self::Api>,
    // ) -> BigUint {
    //     self.rule_of_three_non_zero_result(
    //         farm_token_amount,
    //         &farm_token_attributes.current_farm_amount,
    //         &farm_token_attributes.initial_farming_amount,
    //     )
    // }

    // fn calculate_previously_compounded_rewards(
    //     &self,
    //     farm_token_amount: &BigUint,
    //     farm_token_attributes: &FarmTokenAttributes<Self::Api>,
    // ) -> BigUint {
    //     self.rule_of_three(
    //         farm_token_amount,
    //         &farm_token_attributes.current_farm_amount,
    //         &farm_token_attributes.compounded_reward,
    //     )
    // }

    // fn calculate_new_compound_reward_amount(
    //     &self,
    //     farm_token_amount: &BigUint,
    //     farm_token_attributes: &FarmTokenAttributes<Self::Api>,
    // ) -> BigUint {
    //     self.rule_of_three(
    //         farm_token_amount,
    //         &farm_token_attributes.current_farm_amount,
    //         &farm_token_attributes.compounded_reward,
    //     )
    // }

    fn default_generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
        let mint_function = |token_id: &TokenIdentifier, amount: &BigUint| {
            self.send().esdt_local_mint(token_id, 0, amount);
        };
        let total_reward =
            self.mint_per_block_rewards(&storage_cache.reward_token_id, mint_function);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }
}
