use common_structs::{Epoch, Timestamp};
use math::linear_interpolation;
use timestamp_oracle::epoch_to_timestamp::ProxyTrait as _;
use week_timekeeping::{Week, EPOCHS_IN_WEEK};
use weekly_rewards_splitting::{ClaimProgress, USER_MAX_CLAIM_WEEKS};

use crate::boosted_yields_factors::BoostedYieldsFactors;

multiversx_sc::imports!();

pub struct CalculateRewardsArgs<'a, M: ManagedTypeApi> {
    pub factors: &'a BoostedYieldsFactors<M>,
    pub weekly_reward_amount: &'a BigUint<M>,
    pub user_farm_amount: &'a BigUint<M>,
    pub farm_supply_for_week: &'a BigUint<M>,
    pub energy_amount: &'a BigUint<M>,
    pub total_energy: &'a BigUint<M>,
}

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

pub struct WeekTimestamps {
    pub start: Timestamp,
    pub end: Timestamp,
}

pub const MAX_PERCENT: u64 = 10_000;

#[multiversx_sc::module]
pub trait CustomRewardLogicModule:
    crate::boosted_yields_factors::BoostedYieldsFactorsModule
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
{
    #[only_owner]
    #[endpoint(setTimestampOracleAddress)]
    fn set_timestamp_oracle_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);

        self.timestamp_oracle_address().set(sc_address);
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

    fn calculate_user_boosted_rewards(&self, args: CalculateRewardsArgs<Self::Api>) -> BigUint {
        let max_rewards =
            &args.factors.max_rewards_factor * args.weekly_reward_amount * args.user_farm_amount
                / args.farm_supply_for_week;

        // computed user rewards = total_boosted_rewards *
        // (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) /
        // (energy_const + farm_const)
        let boosted_rewards_by_energy = args.weekly_reward_amount
            * &args.factors.user_rewards_energy_const
            * args.energy_amount
            / args.total_energy;
        let boosted_rewards_by_tokens = args.weekly_reward_amount
            * &args.factors.user_rewards_farm_const
            * args.user_farm_amount
            / args.farm_supply_for_week;
        let constants_base =
            &args.factors.user_rewards_energy_const + &args.factors.user_rewards_farm_const;
        let boosted_reward_amount =
            (boosted_rewards_by_energy + boosted_rewards_by_tokens) / constants_base;

        core::cmp::min(max_rewards, boosted_reward_amount)
    }

    fn limit_boosted_rewards_by_claim_time(
        &self,
        user_reward: BigUint,
        week_timestamps: &WeekTimestamps,
        claim_progress: &ClaimProgress<Self::Api>,
    ) -> BigUint {
        if !(claim_progress.enter_timestamp >= week_timestamps.start
            && claim_progress.enter_timestamp < week_timestamps.end)
        {
            return user_reward;
        }

        // Example: user entered at 25% of week, so give only 75% of rewards
        let enter_timestamp_percent_of_week = linear_interpolation::<Self::Api, _>(
            week_timestamps.start as u128,
            week_timestamps.end as u128,
            claim_progress.enter_timestamp as u128,
            0,
            MAX_PERCENT as u128,
        );
        let percent_leftover = MAX_PERCENT as u128 - enter_timestamp_percent_of_week;

        user_reward * BigUint::from(percent_leftover) / MAX_PERCENT
    }

    fn get_week_start_and_end_timestamp(&self, week: Week) -> WeekTimestamps {
        let week_start_epoch = self.get_start_epoch_for_week(week);
        let week_end_epoch = week_start_epoch + EPOCHS_IN_WEEK;

        let mut needed_epoch_timestamps = MultiValueEncoded::new();
        needed_epoch_timestamps.push(week_start_epoch);
        needed_epoch_timestamps.push(week_end_epoch);

        let timestamps = self
            .get_multiple_epochs_start_timestamp(needed_epoch_timestamps)
            .to_vec();
        let week_start_timestamp = timestamps.get(0);
        let week_end_timestamp = timestamps.get(1) - 1;

        WeekTimestamps {
            start: week_start_timestamp,
            end: week_end_timestamp,
        }
    }

    #[inline]
    fn update_start_of_epoch_timestamp(&self) {
        let _ = self.get_start_of_epoch_timestamp();
    }

    fn get_start_of_epoch_timestamp(&self) -> Timestamp {
        let timestamp_oracle_addr = self.timestamp_oracle_address().get();
        self.timestamp_oracle_proxy_obj(timestamp_oracle_addr)
            .update_and_get_timestamp_start_epoch()
            .execute_on_dest_context()
    }

    fn get_epoch_start_timestamp(&self, epoch: Epoch) -> Timestamp {
        let timestamp_oracle_addr = self.timestamp_oracle_address().get();
        self.timestamp_oracle_proxy_obj(timestamp_oracle_addr)
            .get_start_timestamp_for_epoch(epoch)
            .execute_on_dest_context()
    }

    fn get_multiple_epochs_start_timestamp(
        &self,
        epochs: MultiValueEncoded<Epoch>,
    ) -> MultiValueEncoded<Timestamp> {
        let timestamp_oracle_addr = self.timestamp_oracle_address().get();
        self.timestamp_oracle_proxy_obj(timestamp_oracle_addr)
            .get_start_timestamp_multiple_epochs(epochs)
            .execute_on_dest_context()
    }

    #[proxy]
    fn timestamp_oracle_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> timestamp_oracle::Proxy<Self::Api>;

    #[storage_mapper("timestampOracleAddress")]
    fn timestamp_oracle_address(&self) -> SingleValueMapper<ManagedAddress>;

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
