#![no_std]
#![allow(clippy::from_over_into)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use base_impl_wrapper::FarmStakingWrapper;
use contexts::storage_cache::StorageCache;
use farm::{base_functions::DoubleMultiPayment, MAX_PERCENT};
use farm_base_impl::base_traits_impl::FarmContract;
use fixed_supply_token::FixedSupplyToken;
use multiversx_sc::storage::StorageKey;
use token_attributes::StakingFarmTokenAttributes;

use crate::custom_rewards::{MAX_MIN_UNBOND_EPOCHS, SECONDS_IN_YEAR};

pub mod base_impl_wrapper;
pub mod claim_only_boosted_staking_rewards;
pub mod claim_stake_farm_rewards;
pub mod compound_stake_farm_rewards;
pub mod custom_rewards;
pub mod external_interaction;
pub mod farm_token_roles;
pub mod stake_farm;
pub mod token_attributes;
pub mod unbond_farm;
pub mod unstake_farm;

#[multiversx_sc::contract]
pub trait FarmStaking:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + permissions_hub_module::PermissionsHubModule
    + original_owner_helper::OriginalOwnerHelperModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_token_roles::FarmTokenRolesModule
    + stake_farm::StakeFarmModule
    + claim_stake_farm_rewards::ClaimStakeFarmRewardsModule
    + compound_stake_farm_rewards::CompoundStakeFarmRewardsModule
    + unstake_farm::UnstakeFarmModule
    + unbond_farm::UnbondFarmModule
    + external_interaction::ExternalInteractionsModule
    + claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + farm_boosted_yields::undistributed_rewards::UndistributedRewardsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
{
    #[init]
    fn init(
        &self,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        max_apr: BigUint,
        min_unbond_epochs: u64,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        // farming and reward token are the same
        self.base_farm_init(
            farming_token_id.clone(),
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        require!(max_apr > 0u64, "Invalid max APR percentage");
        self.max_annual_percentage_rewards().set_if_empty(&max_apr);

        require!(
            min_unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            "Invalid min unbond epochs"
        );
        self.min_unbond_epochs().set_if_empty(min_unbond_epochs);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);

        // Initialize last_reward_timestamp
        let current_timestamp = self.blockchain().get_block_timestamp();
        self.last_reward_timestamp().set_if_empty(current_timestamp);

        // Farm position migration code
        let farm_token_mapper = self.farm_token();
        self.try_set_farm_position_migration_nonce(farm_token_mapper);
    }

    #[upgrade]
    fn upgrade(&self) {
        let mut storage_cache = StorageCache::new(self);

        // GENERATE AGGREGATED REWARDS
        let accumulated_rewards_mapper = self.accumulated_rewards();
        let mut accumulated_rewards = accumulated_rewards_mapper.get();
        let reward_capacity = self.reward_capacity().get();
        let remaining_rewards = &reward_capacity - &accumulated_rewards;

        // MINT PER BLOCK REWARDS
        let last_reward_block_nonce_mapper =
            SingleValueMapper::<Self::Api, u64>::new(StorageKey::new(b"last_reward_block_nonce"));
        let per_block_reward_amount_mapper = SingleValueMapper::<Self::Api, BigUint>::new(
            StorageKey::new(b"per_block_reward_amount"),
        );

        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = last_reward_block_nonce_mapper.take();
        let per_block_reward = per_block_reward_amount_mapper.take();

        // CALCULATE PER BLOCK REWARDS
        let extra_rewards_unbounded =
            if current_block_nonce <= last_reward_nonce || !self.produces_per_second_rewards() {
                BigUint::zero()
            } else {
                let block_nonce_diff = current_block_nonce - last_reward_nonce;

                // Self::calculate_per_block_rewards(sc, current_block_nonce, last_reward_nonce);
                &per_block_reward * block_nonce_diff
            };

        let farm_token_supply = self.farm_token_supply().get();
        let max_apr = self.max_annual_percentage_rewards().get();
        let extra_rewards_apr_bounded_per_block =
            farm_token_supply * &max_apr / MAX_PERCENT / SECONDS_IN_YEAR / 6u64; // 6 seconds per block

        let block_nonce_diff = current_block_nonce - last_reward_nonce;
        let extra_rewards_apr_bounded = extra_rewards_apr_bounded_per_block * block_nonce_diff;

        let mut total_reward = core::cmp::min(extra_rewards_unbounded, extra_rewards_apr_bounded);

        // COMPLETE REWARDS GENERATION
        total_reward = core::cmp::min(total_reward, remaining_rewards);
        if total_reward > 0 {
            storage_cache.reward_reserve += &total_reward;
            accumulated_rewards += &total_reward;
            accumulated_rewards_mapper.set(&accumulated_rewards);

            let split_rewards = self.take_reward_slice(total_reward);
            if storage_cache.farm_token_supply > 0 {
                let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }

        // MIGRATE DATA
        let per_second_reward_amount = per_block_reward / 6u64; // 6 seconds per block
        self.per_second_reward_amount()
            .set_if_empty(per_second_reward_amount);
        self.last_reward_timestamp()
            .set_if_empty(self.blockchain().get_block_timestamp());
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> DoubleMultiPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.migrate_old_farm_positions(&caller);

        let boosted_rewards = self.claim_only_boosted_payment(&caller);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let merged_farm_token = self.merge_and_update_farm_tokens(caller.clone());

        self.send_payment_non_zero(&caller, &merged_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);

        (merged_farm_token, boosted_rewards_payment).into()
    }

    fn merge_and_update_farm_tokens(&self, orig_caller: ManagedAddress) -> EsdtTokenPayment {
        let mut output_attributes =
            self.merge_farm_tokens::<FarmStakingWrapper<Self>>(&orig_caller);
        output_attributes.original_owner = orig_caller;

        let new_token_amount = output_attributes.get_total_supply();
        self.farm_token()
            .nft_create(new_token_amount, &output_attributes)
    }

    fn merge_farm_tokens<FC: FarmContract<FarmSc = Self>>(
        &self,
        orig_caller: &ManagedAddress,
    ) -> FC::AttributesType {
        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        token_mapper.require_all_same_token(&payments);

        FC::check_and_update_user_farm_position(self, orig_caller, &payments);

        self.merge_from_payments_and_burn(payments, &token_mapper)
    }

    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_has_admin_permissions();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        self.boosted_yields_rewards_percentage().set(percentage);
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        farm_token_amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        FarmStakingWrapper::<Self>::calculate_rewards(
            self,
            &ManagedAddress::zero(),
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
    }

    fn require_queried(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();
        require!(
            caller == sc_address,
            "May only call this function through VM query"
        );
    }
}
