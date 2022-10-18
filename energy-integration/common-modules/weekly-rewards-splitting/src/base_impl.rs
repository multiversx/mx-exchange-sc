elrond_wasm::imports!();

use common_types::{TokenAmountPairsVec};
use week_timekeeping::Week;

use crate::{events, ClaimProgress};

pub trait AllBaseWeeklyRewardsSplittingImplTraits = crate::WeeklyRewardsSplittingModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + events::WeeklyRewardsSplittingEventsModule;

pub trait WeeklyRewardsSplittingTraitsModule {
    type WeeklyRewardsSplittingMod: AllBaseWeeklyRewardsSplittingImplTraits;

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

    fn collect_rewards_for_week(
        module: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> TokenAmountPairsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>;

    fn get_current_claim_progress(
        module: &Self::WeeklyRewardsSplittingMod,
        user: &ManagedAddress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    ) -> SingleValueMapper<
        <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
        ClaimProgress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    > {
        module.current_claim_progress(user)
    }

    fn get_current_farm_token_nonce(&self) -> u64;

    // fn get_user_rewards_for_week(
    //     module: &Self::WeeklyRewardsSplittingMod,
    //     week: Week,
    //     _farm_token_position_amount: &BigUint<
    //         <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
    //     >,
    //     _user_total_farm_tokens: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    //     energy_amount: &BigUint<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    //     total_rewards: &TokenAmountPairsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    // ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
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

    // fn get_user_energy_for_week(
    //     module: &Self::WeeklyRewardsSplittingMod,
    //     user: &ManagedAddress<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    //     week: Week,
    // ) -> SingleValueMapper<
    //     <Self::WeeklyRewardsSplittingMod as ContractBase>::Api,
    //     Energy<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api>,
    // > {
    //     module.user_energy_for_week(user, week)
    // }

}