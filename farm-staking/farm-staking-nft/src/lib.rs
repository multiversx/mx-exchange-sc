#![no_std]
#![allow(clippy::from_over_into)]
#![feature(trait_alias)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use contexts::storage_cache::StorageCache;
use result_types::MergeResultType;
use token_attributes::PartialStakingFarmNftTokenAttributes;

use crate::custom_rewards::MAX_MIN_UNBOND_EPOCHS;

pub mod custom_rewards;
pub mod farm_actions;
pub mod farm_hooks;
pub mod farm_token_roles;
pub mod result_types;
pub mod token_attributes;
pub mod token_info;

#[multiversx_sc::contract]
pub trait FarmStaking:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_token_roles::FarmTokenRolesModule
    + farm_actions::stake_farm::StakeFarmModule
    + farm_actions::claim_stake_farm_rewards::ClaimStakeFarmRewardsModule
    + farm_actions::compound_stake_farm_rewards::CompoundStakeFarmRewardsModule
    + farm_actions::unstake_farm::UnstakeFarmModule
    + farm_actions::unbond_farm::UnbondFarmModule
    + farm_actions::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + banned_addresses::BannedAddressModule
    + farm_hooks::change_hooks::ChangeHooksModule
    + farm_hooks::call_hook::CallHookModule
    + token_info::TokenInfoModule
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

        let sc_address = self.blockchain().get_sc_address();
        self.banned_addresses().add(&sc_address);
    }

    #[endpoint]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> MergeResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let boosted_rewards = self.claim_only_boosted_payment(&caller);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        let output_attributes = self.merge_from_payments_and_burn_nft(payments, &token_mapper);
        let new_token_amount = output_attributes.current_farm_amount.clone();
        let full_attr = output_attributes.into_full();

        let merged_farm_token = token_mapper.nft_create(new_token_amount, &full_attr);
        self.send_payment_non_zero(&caller, &merged_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);

        MergeResultType {
            merged_farm_token,
            boosted_rewards_payment,
        }
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        farm_token_amount: BigUint,
        attributes: PartialStakingFarmNftTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        self.generate_aggregated_rewards(&mut storage_cache);

        self.calculate_rewards(
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
