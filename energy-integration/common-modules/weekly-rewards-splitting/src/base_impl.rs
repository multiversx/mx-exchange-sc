multiversx_sc::imports!();

use common_types::PaymentsVec;
use week_timekeeping::Week;

use crate::{events, ClaimProgress};

pub trait AllBaseWeeklyRewardsSplittingImplTraits =
    crate::WeeklyRewardsSplittingModule
        + energy_query::EnergyQueryModule
        + week_timekeeping::WeekTimekeepingModule
        + crate::global_info::WeeklyRewardsGlobalInfo
        + crate::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
        + events::WeeklyRewardsSplittingEventsModule
        + crate::update_claim_progress_energy::UpdateClaimProgressEnergyModule;

pub trait WeeklyRewardsSplittingTraitsModule {
    type WeeklyRewardsSplittingMod: AllBaseWeeklyRewardsSplittingImplTraits;

    fn collect_and_get_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let total_rewards_mapper = sc.total_rewards_for_week(week);
        if total_rewards_mapper.is_empty() {
            let total_rewards = self.collect_rewards_for_week(sc, week);
            total_rewards_mapper.set(&total_rewards);

            total_rewards
        } else {
            total_rewards_mapper.get()
        }
    }

    fn get_user_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
        energy_amount: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
        total_energy: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let mut user_rewards = ManagedVec::new();
        if energy_amount == &0 || total_energy == &0 {
            return user_rewards;
        }

        let total_rewards = self.collect_and_get_rewards_for_week(sc, week);
        for weekly_reward in &total_rewards {
            let reward_amount = weekly_reward.amount * energy_amount / total_energy;
            if reward_amount > 0 {
                user_rewards.push(EsdtTokenPayment::new(
                    weekly_reward.token_identifier,
                    0,
                    reward_amount,
                ));
            }
        }

        user_rewards
    }

    fn collect_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>;

    fn get_claim_progress_mapper(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        user: &ManagedAddress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> SingleValueMapper<
        <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
        ClaimProgress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    > {
        sc.current_claim_progress(user)
    }
}
