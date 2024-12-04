multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{Epoch, Percent};
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_functions::FarmWithTopUpWrapper;

// TODO: Will need to be changed when block duration changes
pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block

pub const MAX_PERCENT: Percent = 10_000;
pub const MAX_MIN_UNBOND_EPOCHS: Epoch = 30;
pub static WITHDRAW_AMOUNT_TOO_HIGH: &[u8] =
    b"Withdraw amount is higher than the remaining uncollected rewards";

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + farm_boosted_yields::custom_reward_logic::CustomRewardLogicModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule // + crate::base_functions::BaseFunctionsModule
{
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let reward_token_id = self.reward_token_id().get();
        require!(payment_token == reward_token_id, "Invalid token");

        self.reward_capacity().update(|r| *r += payment_amount);

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(withdrawRewards)]
    fn withdraw_rewards(&self, withdraw_amount: BigUint) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        FarmWithTopUpWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let reward_capactiy = self.reward_capacity().get();
        let accumulated_rewards = self.accumulated_rewards().get();
        let remaining_rewards = &reward_capactiy - &accumulated_rewards;
        require!(
            withdraw_amount <= remaining_rewards,
            WITHDRAW_AMOUNT_TOO_HIGH
        );
        require!(
            reward_capactiy >= withdraw_amount,
            "Not enough rewards to withdraw"
        );

        let new_capacity = &reward_capactiy - &withdraw_amount;
        self.reward_capacity().set(new_capacity);

        let caller = self.blockchain().get_caller();
        let reward_token_id = self.reward_token_id().get();
        self.send().direct_non_zero(
            &caller,
            &EgldOrEsdtTokenIdentifier::esdt(reward_token_id),
            0,
            &withdraw_amount,
        );

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        FarmWithTopUpWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.produce_rewards_enabled().set(false);

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(per_block_amount != 0, "Amount cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        FarmWithTopUpWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.per_block_reward_amount().set(&per_block_amount);

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: Percent) {
        self.require_caller_has_admin_permissions();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        let mut storage_cache = StorageCache::new(self);
        FarmWithTopUpWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        self.boosted_yields_rewards_percentage().set(percentage);

        self.update_start_of_epoch_timestamp();
    }

    #[view(getAccumulatedRewards)]
    #[storage_mapper("accumulatedRewards")]
    fn accumulated_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardCapacity)]
    #[storage_mapper("reward_capacity")]
    fn reward_capacity(&self) -> SingleValueMapper<BigUint>;
}
