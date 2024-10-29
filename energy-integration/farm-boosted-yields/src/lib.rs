#![no_std]

multiversx_sc::imports!();

use boosted_yields_factors::BoostedYieldsConfig;
use common_structs::PaymentsVec;
use custom_reward_logic::CalculateRewardsArgs;
use multiversx_sc::api::ErrorApi;
use week_timekeeping::Week;
use weekly_rewards_splitting::{base_impl::WeeklyRewardsSplittingTraitsModule, ClaimProgress};

pub mod boosted_yields_factors;
pub mod custom_reward_logic;

#[multiversx_sc::module]
pub trait FarmBoostedYieldsModule:
    boosted_yields_factors::BoostedYieldsFactorsModule
    + config::ConfigModule
    + week_timekeeping::WeekTimekeepingModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + custom_reward_logic::CustomRewardLogicModule
{
    fn claim_boosted_yields_rewards(
        &self,
        user: &ManagedAddress,
        farm_token_amount: BigUint,
    ) -> BigUint {
        let opt_config = self.try_get_boosted_yields_config();
        let config = match opt_config {
            Some(c) => c,
            None => {
                return BigUint::zero();
            }
        };
        let wrapper = FarmBoostedYieldsWrapper::new(farm_token_amount, config);
        let rewards = self.claim_multi(&wrapper, user);

        let mut total = BigUint::zero();
        for rew in &rewards {
            total += rew.amount;
        }

        total
    }

    fn set_farm_supply_for_current_week(&self, farm_supply: &BigUint) {
        let current_week = self.get_current_week();
        self.farm_supply_for_week(current_week).set(farm_supply);
    }

    fn clear_user_energy_if_needed(&self, original_caller: &ManagedAddress) {
        let opt_config = self.try_get_boosted_yields_config();
        let user_total_farm_position = self.user_total_farm_position(original_caller).get();
        if let Some(config) = opt_config {
            let boosted_yields_factors = config.get_latest_factors();
            self.clear_user_energy(
                original_caller,
                &user_total_farm_position,
                &boosted_yields_factors.min_farm_amount,
            );
        }
    }
}

pub struct FarmBoostedYieldsWrapper<T: FarmBoostedYieldsModule> {
    pub user_farm_amount: BigUint<<T as ContractBase>::Api>,
    pub boosted_yields_config: BoostedYieldsConfig<<T as ContractBase>::Api>,
}

impl<T: FarmBoostedYieldsModule> FarmBoostedYieldsWrapper<T> {
    pub fn new(
        user_farm_amount: BigUint<<T as ContractBase>::Api>,
        boosted_yields_config: BoostedYieldsConfig<<T as ContractBase>::Api>,
    ) -> FarmBoostedYieldsWrapper<T> {
        FarmBoostedYieldsWrapper {
            user_farm_amount,
            boosted_yields_config,
        }
    }
}

impl<T> WeeklyRewardsSplittingTraitsModule for FarmBoostedYieldsWrapper<T>
where
    T: FarmBoostedYieldsModule,
{
    type WeeklyRewardsSplittingMod = T;

    fn collect_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        sc.update_boosted_yields_config();

        let reward_token_id = sc.reward_token_id().get();
        let total_rewards = sc.accumulated_rewards_for_week(week).take();
        sc.remaining_boosted_rewards_to_distribute(week)
            .set(&total_rewards);

        ManagedVec::from_single_item(EsdtTokenPayment::new(reward_token_id, 0, total_rewards))
    }

    fn get_user_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        claim_progress: &mut ClaimProgress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
        total_energy: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let mut user_rewards = ManagedVec::new();
        let energy_amount = claim_progress.energy.get_energy_amount();
        let farm_supply_for_week = sc.farm_supply_for_week(claim_progress.week).get();

        let current_week = sc.get_current_week();
        let week_timestamps = sc.get_week_start_and_end_timestamp(claim_progress.week + 1);
        let current_timestamp = sc.blockchain().get_block_timestamp();
        let min_timestamp = core::cmp::min(current_timestamp, week_timestamps.end);

        if total_energy == &0 || farm_supply_for_week == 0 {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        let factors = self
            .boosted_yields_config
            .get_factors_for_week(claim_progress.week);
        if energy_amount < factors.min_energy_amount
            || self.user_farm_amount < factors.min_farm_amount
        {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        let total_rewards = self.collect_and_get_rewards_for_week(sc, claim_progress.week);
        if total_rewards.is_empty() {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        // always no entries or 1 entry, but the trait uses a Vec. Just a sanity check.
        if total_rewards.len() != 1 {
            <<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>::error_api_impl()
                .signal_error(b"Invalid boosted yields rewards");
        }

        let weekly_reward = total_rewards.get(0);
        if weekly_reward.amount == 0 {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        let user_reward = sc.calculate_user_boosted_rewards(CalculateRewardsArgs {
            factors,
            weekly_reward_amount: &weekly_reward.amount,
            user_farm_amount: &self.user_farm_amount,
            farm_supply_for_week: &farm_supply_for_week,
            energy_amount: &energy_amount,
            total_energy,
        });
        if user_reward == 0 {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        let new_user_reward =
            sc.limit_boosted_rewards_by_claim_time(user_reward, &week_timestamps, claim_progress);
        if new_user_reward == 0 {
            let _ = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);

            return user_rewards;
        }

        let prev_week = sc.advance_week_if_needed(current_week, min_timestamp, claim_progress);
        sc.remaining_boosted_rewards_to_distribute(prev_week)
            .update(|amount| *amount -= &new_user_reward);

        user_rewards.push(EsdtTokenPayment::new(
            weekly_reward.token_identifier,
            0,
            new_user_reward,
        ));

        user_rewards
    }
}
