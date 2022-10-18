#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use common_types::{Nonce, TokenAmountPair, TokenAmountPairsVec};
use week_timekeeping::Week;
use weekly_rewards_splitting::{base_impl::WeeklyRewardsSplittingTraitsModule, ClaimProgress};

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
    + permissions_module::PermissionsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_has_admin_permissions();
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

    fn claim_boosted_yields_rewards(
        &self,
        user: &ManagedAddress,
        _farm_token_nonce: Nonce,
        _reward_token_id: &TokenIdentifier,
    ) -> BigUint {
        let rewards = self.claim_multi::<FarmBoostedYieldsWrapper<Self>>(&user);

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

    #[storage_mapper("farmClaimProgress")]
    fn farm_claim_progress(
        &self,
        user: &ManagedAddress,
        token_nonce: Nonce,
    ) -> SingleValueMapper<ClaimProgress<Self::Api>>;
}

pub struct FarmBoostedYieldsWrapper<T: FarmBoostedYieldsModule> {
    pub current_farm_token_nonce: Nonce,
    pub phantom: PhantomData<T>,
}

impl<T> WeeklyRewardsSplittingTraitsModule for FarmBoostedYieldsWrapper<T>
where
    T: FarmBoostedYieldsModule,
{
    type WeeklyRewardsSplittingMod = T;

    fn get_current_farm_token_nonce(&self) -> u64 {
        self.current_farm_token_nonce
    }

    // TODO - token identifier?
    fn collect_rewards_for_week(
        module: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> TokenAmountPairsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let rewards_mapper = module.accumulated_rewards_for_week(week);
        let total_rewards = rewards_mapper.get();
        rewards_mapper.clear();

        ManagedVec::from_single_item(TokenAmountPair::new(
            TokenIdentifier::from(ManagedBuffer::new()),
            total_rewards,
        ))
    }

    // TODO - add token nonce
    fn get_current_claim_progress(
        module: &Self::WeeklyRewardsSplittingMod,
        user: &ManagedAddress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> SingleValueMapper<
        <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
        ClaimProgress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    > {
        // let token_nonce = Self::get_current_farm_token_nonce(module);
        module.farm_claim_progress(user, 0u64)
    }

    fn collect_and_get_rewards_for_week_base(
        module: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> TokenAmountPairsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let total_rewards_mapper = module.total_rewards_for_week(week);
        if total_rewards_mapper.is_empty() {
            let total_rewards = Self::collect_rewards_for_week(module, week);
            total_rewards_mapper.set(&total_rewards);

            total_rewards
        } else {
            total_rewards_mapper.get()
        }
    }

    // fn get_user_rewards_for_week(
    //     module: &Self::WeeklyRewardsSplittingMod,
    //     week: Week,
    //     _farm_token_position_amount: &BigUint<
    //         <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
    //     >,
    //     _user_total_farm_tokens: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    //     energy_amount: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    //     total_rewards: &TokenAmountPairsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    // ) -> common_types::PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
    //     let mut user_rewards = ManagedVec::new();
    //     if energy_amount == &0 {
    //         return user_rewards;
    //     }

    //     let total_energy = module.total_energy_for_week(week).get();
    //     for weekly_reward in total_rewards {
    //         let reward_amount = weekly_reward.amount * energy_amount / &total_energy;
    //         if reward_amount > 0 {
    //             user_rewards.push(EsdtTokenPayment::new(weekly_reward.token, 0, reward_amount));
    //         }
    //     }

    //     user_rewards
    // }
}
