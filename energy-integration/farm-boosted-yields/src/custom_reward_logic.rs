use common_structs::{Epoch, Nonce, Timestamp};
use timestamp_oracle::epoch_to_timestamp::ProxyTrait as _;
use week_timekeeping::Week;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

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
    #[endpoint(setTimestampOracleAddress)]
    fn set_timestamp_oracle_address(&self, sc_address: ManagedAddress) {
        self.require_caller_has_admin_permissions();

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

    fn get_start_of_epoch_timestamp(&self) -> Timestamp {
        let timestamp_oracle_addr = self.timestamp_oracle_address().get();
        self.timestamp_oracle_proxy_obj(timestamp_oracle_addr)
            .update_and_get_timestamp_start_epoch()
            .execute_on_dest_context()
    }

    fn get_custom_epoch_start_timestamp(&self, epoch: Epoch) -> Timestamp {
        let timestamp_oracle_addr = self.timestamp_oracle_address().get();
        self.timestamp_oracle_proxy_obj(timestamp_oracle_addr)
            .get_start_timestamp_for_epoch(epoch)
            .execute_on_dest_context()
    }

    #[proxy]
    fn timestamp_oracle_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> timestamp_oracle::Proxy<Self::Api>;

    #[storage_mapper("timestampOracleAddress")]
    fn timestamp_oracle_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("posEnterTimestamp")]
    fn pos_enter_timestamp(&self, pos_nonce: Nonce) -> SingleValueMapper<Timestamp>;

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
