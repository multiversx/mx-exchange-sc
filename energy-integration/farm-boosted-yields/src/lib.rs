#![no_std]

multiversx_sc::imports!();

use core::cmp;

use boosted_yields_factors::BoostedYieldsConfig;
use common_types::PaymentsVec;
use multiversx_sc::api::ErrorApi;
use week_timekeeping::Week;
use weekly_rewards_splitting::{
    base_impl::WeeklyRewardsSplittingTraitsModule, USER_MAX_CLAIM_WEEKS,
};

pub mod boosted_yields_factors;

const MAX_PERCENT: u64 = 10_000;

pub struct SplitReward<M: ManagedTypeApi> {
    pub base_farm: BigUint<M>,
    pub boosted_farm: BigUint<M>,
}

impl<M: ManagedTypeApi> SplitReward<M> {
    pub fn new(base_farm: BigUint<M>, boosted_farm: BigUint<M>) -> Self {
        SplitReward {
            base_farm,
            boosted_farm,
        }
    }
}

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
{
    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_has_admin_permissions();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        self.boosted_yields_rewards_percentage().set(percentage);
    }

    #[endpoint(collectUndistributedBoostedRewards)]
    fn collect_undistributed_boosted_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1usize;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            "Current week must be higher than the week offset"
        );

        let last_collect_week_mapper = self.last_undistributed_boosted_rewards_collect_week();
        let first_collect_week = last_collect_week_mapper.get() + 1;
        let last_collect_week = current_week - collect_rewards_offset;
        if first_collect_week > last_collect_week {
            return;
        }

        for week in first_collect_week..=last_collect_week {
            let rewards_to_distribute = self.remaining_boosted_rewards_to_distribute(week).take();
            self.undistributed_boosted_rewards()
                .update(|total_amount| *total_amount += rewards_to_distribute);
        }

        last_collect_week_mapper.set(last_collect_week);
    }

    fn take_reward_slice(&self, full_reward: BigUint) -> SplitReward<Self::Api> {
        let percentage = self.boosted_yields_rewards_percentage().get();
        if percentage == 0 {
            return SplitReward::new(full_reward, BigUint::zero());
        }

        let boosted_yields_cut = &full_reward * percentage / MAX_PERCENT;
        let base_farm_amount = if boosted_yields_cut > 0 {
            let current_week = self.get_current_week();
            self.accumulated_rewards_for_week(current_week)
                .update(|accumulated_rewards| {
                    *accumulated_rewards += &boosted_yields_cut;
                });

            &full_reward - &boosted_yields_cut
        } else {
            full_reward
        };

        SplitReward::new(base_farm_amount, boosted_yields_cut)
    }

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

    fn clear_user_energy_if_needed(
        &self,
        original_caller: &ManagedAddress,
        user_remaining_farm_tokens: &BigUint,
    ) {
        let opt_config = self.try_get_boosted_yields_config();
        if let Some(config) = opt_config {
            let boosted_yields_factors = config.get_latest_factors();
            self.clear_user_energy(
                original_caller,
                user_remaining_farm_tokens,
                &boosted_yields_factors.min_farm_amount,
            );
        }
    }

    #[view(getBoostedYieldsRewardsPercentage)]
    #[storage_mapper("boostedYieldsRewardsPercentage")]
    fn boosted_yields_rewards_percentage(&self) -> SingleValueMapper<u64>;

    #[view(getAccumulatedRewardsForWeek)]
    #[storage_mapper("accumulatedRewardsForWeek")]
    fn accumulated_rewards_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getFarmSupplyForWeek)]
    #[storage_mapper("farmSupplyForWeek")]
    fn farm_supply_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getRemainingBoostedRewardsToDistribute)]
    #[storage_mapper("remainingBoostedRewardsToDistribute")]
    fn remaining_boosted_rewards_to_distribute(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("lastUndistributedBoostedRewardsCollectWeek")]
    fn last_undistributed_boosted_rewards_collect_week(&self) -> SingleValueMapper<Week>;

    #[view(getUndistributedBoostedRewards)]
    #[storage_mapper("undistributedBoostedRewards")]
    fn undistributed_boosted_rewards(&self) -> SingleValueMapper<BigUint>;
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
        week: Week,
        energy_amount: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
        total_energy: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let mut user_rewards = ManagedVec::new();
        let farm_supply_for_week = sc.farm_supply_for_week(week).get();
        if total_energy == &0 || farm_supply_for_week == 0 {
            return user_rewards;
        }

        let factors = self.boosted_yields_config.get_factors_for_week(week);
        if energy_amount < &factors.min_energy_amount
            || self.user_farm_amount < factors.min_farm_amount
        {
            return user_rewards;
        }

        let total_rewards = self.collect_and_get_rewards_for_week(sc, week);
        if total_rewards.is_empty() {
            return user_rewards;
        }

        // always no entries or 1 entry, but the trait uses a Vec
        if total_rewards.len() != 1 {
            <<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>::error_api_impl()
                .signal_error(b"Invalid boosted yields rewards");
        }

        let weekly_reward = total_rewards.get(0);
        if weekly_reward.amount == 0 {
            return user_rewards;
        }

        let max_rewards =
            &factors.max_rewards_factor * &weekly_reward.amount * &self.user_farm_amount
                / &farm_supply_for_week;

        // computed user rewards = total_boosted_rewards *
        // (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) /
        // (energy_const + farm_const)
        let boosted_rewards_by_energy =
            &weekly_reward.amount * &factors.user_rewards_energy_const * energy_amount
                / total_energy;
        let boosted_rewards_by_tokens =
            &weekly_reward.amount * &factors.user_rewards_farm_const * &self.user_farm_amount
                / &farm_supply_for_week;
        let constants_base = &factors.user_rewards_energy_const + &factors.user_rewards_farm_const;
        let boosted_reward_amount =
            (boosted_rewards_by_energy + boosted_rewards_by_tokens) / constants_base;

        // min between base rewards per week and computed rewards
        let user_reward = cmp::min(max_rewards, boosted_reward_amount);
        if user_reward > 0 {
            sc.remaining_boosted_rewards_to_distribute(week)
                .update(|amount| *amount -= &user_reward);

            user_rewards.push(EsdtTokenPayment::new(
                weekly_reward.token_identifier,
                0,
                user_reward,
            ));
        }

        user_rewards
    }
}
