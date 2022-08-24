#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_types::{TokenAmountPair, TokenAmountPairsVec};
use week_timekeeping::Week;

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

#[elrond_wasm::module]
pub trait FarmBoostedYieldsModule:
    week_timekeeping::WeekTimekeepingModule
    + admin_whitelist::AdminWhitelistModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_is_admin();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        self.boosted_yields_rewards_percentage().set(percentage);
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

    fn collect_rewards(
        &self,
        week: Week,
        reward_token_id: &TokenIdentifier,
    ) -> TokenAmountPairsVec<Self::Api> {
        let rewards_mapper = self.accumulated_rewards_for_week(week);
        let total_rewards = rewards_mapper.get();
        rewards_mapper.clear();

        ManagedVec::from_single_item(TokenAmountPair::new(reward_token_id.clone(), total_rewards))
    }

    fn claim_boosted_yields_rewards(
        &self,
        user: &ManagedAddress,
        reward_token_id: &TokenIdentifier,
    ) -> BigUint {
        let rewards = self.claim_multi(user, |sc_ref: &Self, week: Week| {
            Self::collect_rewards(sc_ref, week, reward_token_id)
        });

        let mut total = BigUint::zero();
        for rew in &rewards {
            total += rew.amount;
        }

        total
    }

    #[view(getBoostedYieldsRewardsPercenatage)]
    #[storage_mapper("boostedYieldsRewardsPercentage")]
    fn boosted_yields_rewards_percentage(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("accumulatedRewardsForWeek")]
    fn accumulated_rewards_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;
}
