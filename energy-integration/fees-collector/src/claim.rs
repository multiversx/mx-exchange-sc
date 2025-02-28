use core::marker::PhantomData;

use common_types::{PaymentsVec, Week};
use weekly_rewards_splitting::base_impl::WeeklyRewardsSplittingTraitsModule;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ClaimModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + crate::fees_accumulation::FeesAccumulationModule
    + crate::additional_locked_tokens::AdditionalLockedTokensModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::pause::PauseModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + crate::redistribute_rewards::RedistributeRewardsModule
{
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> PaymentsVec<Self::Api> {
        self.require_not_paused();

        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);

        self.claim_rewards(caller, original_caller)
    }

    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> PaymentsVec<Self::Api> {
        self.require_not_paused();

        let original_caller = match opt_original_caller {
            OptionalValue::Some(user) => {
                require!(
                    self.allow_external_claim_rewards(&user).get(),
                    "Cannot claim rewards for this address"
                );

                user
            }
            OptionalValue::None => self.blockchain().get_caller(),
        };

        self.claim_rewards(original_caller.clone(), original_caller)
    }

    fn claim_rewards(
        &self,
        caller: ManagedAddress,
        original_caller: ManagedAddress,
    ) -> PaymentsVec<Self::Api> {
        self.accumulate_additional_locked_tokens();

        let wrapper = FeesCollectorWrapper::new();
        let mut rewards = self.claim_multi(&wrapper, &original_caller);
        if rewards.is_empty() {
            return rewards;
        }

        let locked_token_id = self.get_locked_token_id();
        let mut i = 0;
        let mut len = rewards.len();
        let mut total_locked_token_rewards_amount = BigUint::zero();
        while i < len {
            let rew = rewards.get(i);
            if rew.token_identifier != locked_token_id {
                i += 1;
                continue;
            }

            total_locked_token_rewards_amount += rew.amount;
            len -= 1;
            rewards.remove(i);
        }

        if !rewards.is_empty() {
            self.send().direct_multi(&caller, &rewards);
        }

        if total_locked_token_rewards_amount > 0 {
            let locked_rewards = self.lock_virtual(
                self.get_base_token_id(),
                total_locked_token_rewards_amount,
                caller,
                original_caller,
            );

            rewards.push(locked_rewards);
        }

        rewards
    }
}

pub struct FeesCollectorWrapper<T: ClaimModule> {
    phantom: PhantomData<T>,
}

impl<T: ClaimModule> Default for FeesCollectorWrapper<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ClaimModule> FeesCollectorWrapper<T> {
    pub fn new() -> FeesCollectorWrapper<T> {
        FeesCollectorWrapper {
            phantom: PhantomData,
        }
    }
}

impl<T> WeeklyRewardsSplittingTraitsModule for FeesCollectorWrapper<T>
where
    T: ClaimModule,
{
    type WeeklyRewardsSplittingMod = T;

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
        let remaining_rewards_mapper = sc.remaining_rewards(week);
        let mut remaining_rewards = remaining_rewards_mapper.get();
        for (i, weekly_reward) in total_rewards.iter().enumerate() {
            let reward_amount = weekly_reward.amount * energy_amount / total_energy;
            if reward_amount == 0 {
                continue;
            }

            let mut rem_rew_entry = remaining_rewards.get_mut(i);
            rem_rew_entry.amount -= &reward_amount;

            user_rewards.push(EsdtTokenPayment::new(
                weekly_reward.token_identifier,
                0,
                reward_amount,
            ));
        }

        remaining_rewards_mapper.set(remaining_rewards);

        user_rewards
    }

    fn collect_and_get_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let total_rewards_mapper = sc.total_rewards_for_week(week);
        if total_rewards_mapper.is_empty() {
            let total_rewards = self.collect_rewards_for_week(sc, week);
            total_rewards_mapper.set(&total_rewards);
            sc.remaining_rewards(week).set(&total_rewards);

            total_rewards
        } else {
            total_rewards_mapper.get()
        }
    }

    fn collect_rewards_for_week(
        &self,
        sc: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let mut results = ManagedVec::new();
        let all_tokens = sc.all_tokens().get();
        for token in &all_tokens {
            let opt_accumulated_fees = sc.get_and_clear_accumulated_fees(week, &token);
            if let Some(accumulated_fees) = opt_accumulated_fees {
                results.push(EsdtTokenPayment::new(token, 0, accumulated_fees));
            }
        }

        results
    }
}
