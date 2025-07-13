multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

pub const MAX_PERCENT: u64 = 10_000;
pub const SECONDS_IN_YEAR: u64 = 31_536_000;
pub const MAX_MIN_UNBOND_EPOCHS: u64 = 30;
pub const WITHDRAW_AMOUNT_TOO_HIGH: &str =
    "Withdraw amount is higher than the remaining uncollected rewards!";

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let reward_token_id = self.reward_token_id().get();
        require!(payment_token == reward_token_id, "Invalid token");

        self.reward_capacity().update(|r| *r += payment_amount);
    }

    #[payable("*")]
    #[endpoint(withdrawRewards)]
    fn withdraw_rewards(&self, withdraw_amount: BigUint) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let reward_capacity_mapper = self.reward_capacity();
        let accumulated_rewards_mapper = self.accumulated_rewards();
        let remaining_rewards = reward_capacity_mapper.get() - accumulated_rewards_mapper.get();
        require!(
            withdraw_amount <= remaining_rewards,
            WITHDRAW_AMOUNT_TOO_HIGH
        );

        reward_capacity_mapper.update(|rewards| {
            require!(
                *rewards >= withdraw_amount,
                "Not enough rewards to withdraw"
            );

            *rewards -= withdraw_amount.clone()
        });

        let caller = self.blockchain().get_caller();
        let reward_token_id = self.reward_token_id().get();
        self.send_tokens_non_zero(&caller, &reward_token_id, 0, &withdraw_amount);
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.produce_rewards_enabled().set(false);
    }

    #[endpoint(setPerSecondRewardAmount)]
    fn set_per_second_rewards(&self, per_second_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(per_second_amount != 0, "Amount cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.per_second_reward_amount().set(&per_second_amount);
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
        amount * &max_apr / MAX_PERCENT / SECONDS_IN_YEAR
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
