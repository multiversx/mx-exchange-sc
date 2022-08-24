#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod exit_penalty;

use common_errors::ERROR_ZERO_AMOUNT;
use common_structs::{FarmTokenAttributes, PaymentsVec};
use contexts::storage_cache::StorageCache;

use exit_penalty::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
};
use farm_base_impl::exit_farm::InternalExitFarmResult;
use week_timekeeping::Week;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
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
    + exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::partial_positions::PartialPositionsModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    // farm boosted yields
    + farm_boosted_yields::FarmBoostedYieldsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + energy_query::EnergyQueryModule
{
    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            admins,
        );

        self.penalty_percent().set_if_empty(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.pair_contract_address().set(&pair_contract_address);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(&self) -> EnterFarmResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_enter_farm_result = self.enter_farm_base(
            payments,
            Self::generate_aggregated_rewards_with_boosted_yields,
            Self::default_create_enter_farm_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        );

        let caller = self.blockchain().get_caller();
        let output_farm_token_payment = base_enter_farm_result.new_farm_token.payment;
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        // self.emit_enter_farm_event(
        //     enter_farm_context.farming_token_payment,
        //     new_farm_token,
        //     created_with_merge,
        //     storage_cache,
        // );

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_claim_rewards_result = self.claim_rewards_base(
            payments,
            Self::generate_aggregated_rewards_with_boosted_yields,
            Self::default_calculate_reward,
            Self::default_create_claim_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        );

        let caller = self.blockchain().get_caller();
        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment;
        let rewards_payment = base_claim_rewards_result.rewards;
        self.send_payment_non_zero(&caller, &output_farm_token_payment);
        self.send_payment_non_zero(&caller, &rewards_payment);

        // self.emit_claim_rewards_event(
        //     claim_rewards_context,
        //     new_farm_token,
        //     created_with_merge,
        //     reward_payment.clone(),
        //     storage_cache,
        // );

        (output_farm_token_payment, rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> CompoundRewardsResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_compound_rewards_result = self.compound_rewards_base(
            payments,
            Self::generate_aggregated_rewards_with_boosted_yields,
            Self::default_calculate_reward,
            Self::default_create_compound_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        );

        let caller = self.blockchain().get_caller();
        let output_farm_token_payment = base_compound_rewards_result.new_farm_token.payment;
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        // self.emit_compound_rewards_event(
        //     compound_rewards_context,
        //     new_farm_token,
        //     created_with_merge,
        //     reward,
        //     storage_cache,
        // );

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(&self) -> ExitFarmResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
        let base_exit_farm_result: InternalExitFarmResult<Self, FarmTokenAttributes<Self::Api>> = self.exit_farm_base(
            payment,
            Self::generate_aggregated_rewards_with_boosted_yields,
            Self::default_calculate_reward,
        );

        let caller = self.blockchain().get_caller();
        let mut farming_token_payment = base_exit_farm_result.farming_token_payment;
        let reward_payment = base_exit_farm_result.reward_payment;

        let initial_farm_token = base_exit_farm_result.context.farm_token;
        if self.should_apply_penalty(initial_farm_token.attributes.entering_epoch) {
            self.burn_penalty(
                &mut farming_token_payment.amount,
                &base_exit_farm_result.storage_cache.farming_token_id,
                &base_exit_farm_result.storage_cache.reward_token_id,
            );
        }

        self.send_payment_non_zero(&caller, &farming_token_payment);
        self.send_payment_non_zero(&caller, &reward_payment);

        // self.emit_exit_farm_event(
        //         exit_farm_context,
        //         farming_token_payment.clone(),
        //         reward_payment.clone(),
        //         storage_cache,
        //     );

        (farming_token_payment, reward_payment).into()
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        let mut storage_cache = StorageCache::new(self);
        self.generate_aggregated_rewards_with_boosted_yields(&mut storage_cache);

        self.default_calculate_reward(&amount, &attributes, &storage_cache)
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();

        let attrs = self.get_default_merged_farm_token_attributes(&payments, Option::None);
        let farm_token_id = self.farm_token().get_token_id();
        self.burn_farm_tokens_from_payments(&payments);

        let new_tokens =
            self.mint_farm_tokens(farm_token_id, attrs.current_farm_amount.clone(), &attrs);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &new_tokens);

        new_tokens
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_is_admin();
        self.start_produce_rewards();
    }

    #[endpoint]
    fn end_produce_rewards(&self) {
        self.require_caller_is_admin();

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards_with_boosted_yields(&mut storage);
        self.produce_rewards_enabled().set(false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_caller_is_admin();
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        let mut storage = StorageCache::new(self);

        self.generate_aggregated_rewards_with_boosted_yields(&mut storage);
        self.per_block_reward_amount().set(&per_block_amount);
    }

    #[endpoint(claimBoostedYieldsRewards)]
    fn claim_boosted_yields_rewards(&self) -> PaymentsVec<Self::Api> {
        let reward_token_id = self.reward_token_id().get();
        self.claim_multi(|sc_ref: &Self, week: Week| Self::collect_rewards(sc_ref, week, &reward_token_id))
    }

    fn generate_aggregated_rewards_with_boosted_yields(&self, storage_cache: &mut StorageCache<Self>) {
        let mint_function = |token_id: &TokenIdentifier, amount: &BigUint| {
            self.send().esdt_local_mint(token_id, 0, amount);
        };
        let total_reward =
            self.mint_per_block_rewards(&storage_cache.reward_token_id, mint_function);

        if total_reward > 0u64 {
            let split_rewards = self.take_reward_slice(total_reward);
            storage_cache.reward_reserve += &split_rewards.base_farm;

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }
}
