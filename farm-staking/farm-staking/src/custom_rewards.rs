multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

pub const MAX_PERCENT: u64 = 10_000;
pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block
const MAX_MIN_UNBOND_EPOCHS: u64 = 30;

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
{
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let reward_token_id = self.reward_token_id().get();
        require!(payment_token == reward_token_id, "Invalid token");

        self.reward_capacity().update(|r| *r += payment_amount);
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.produce_rewards_enabled().set(false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(per_block_amount != 0, "Amount cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.per_block_reward_amount().set(&per_block_amount);
    }

    #[endpoint(setMaxApr)]
    fn set_max_apr(&self, max_apr: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(max_apr != 0, "Max APR cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.max_annual_percentage_rewards().set(&max_apr);
    }

    #[endpoint(setMinUnbondEpochs)]
    fn set_min_unbond_epochs_endpoint(&self, min_unbond_epochs: Epoch) {
        self.require_caller_has_admin_permissions();
        self.try_set_min_unbond_epochs(min_unbond_epochs);
    }

    fn try_set_min_unbond_epochs(&self, min_unbond_epochs: Epoch) {
        require!(
            min_unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            "Invalid min unbond epochs"
        );

        self.min_unbond_epochs().set(min_unbond_epochs);
    }

    fn get_amount_apr_bounded(&self, amount: &BigUint) -> BigUint {
        let max_apr = self.max_annual_percentage_rewards().get();
        amount * &max_apr / MAX_PERCENT / BLOCKS_IN_YEAR
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[view(getAccumulatedRewards)]
    #[storage_mapper("accumulatedRewards")]
    fn accumulated_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardCapacity)]
    #[storage_mapper("reward_capacity")]
    fn reward_capacity(&self) -> SingleValueMapper<BigUint>;

    #[view(getAnnualPercentageRewards)]
    #[storage_mapper("annualPercentageRewards")]
    fn max_annual_percentage_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinUnbondEpochs)]
    #[storage_mapper("minUnbondEpochs")]
    fn min_unbond_epochs(&self) -> SingleValueMapper<Epoch>;
}
