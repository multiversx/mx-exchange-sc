#![no_std]

multiversx_sc::imports!();

use common_types::{PaymentsVec, Week};
use core::marker::PhantomData;
use weekly_rewards_splitting::base_impl::WeeklyRewardsSplittingTraitsModule;

pub mod additional_locked_tokens;
pub mod config;
pub mod events;
pub mod fees_accumulation;

#[multiversx_sc::contract]
pub trait FeesCollector:
    config::ConfigModule
    + events::FeesCollectorEventsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + fees_accumulation::FeesAccumulationModule
    + additional_locked_tokens::AdditionalLockedTokensModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::pause::PauseModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[init]
    fn init(&self, locked_token_id: TokenIdentifier, energy_factory_address: ManagedAddress) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);
        self.require_valid_token_id(&locked_token_id);
        self.require_sc_address(&energy_factory_address);

        let mut tokens = MultiValueEncoded::new();
        tokens.push(locked_token_id.clone());
        self.add_known_tokens(tokens);

        self.locked_token_id().set_if_empty(locked_token_id);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> PaymentsVec<Self::Api> {
        require!(self.not_paused(), "Cannot claim while paused");

        self.accumulate_additional_locked_tokens();

        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);

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

pub struct FeesCollectorWrapper<T: FeesCollector> {
    phantom: PhantomData<T>,
}

impl<T: FeesCollector> Default for FeesCollectorWrapper<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FeesCollector> FeesCollectorWrapper<T> {
    pub fn new() -> FeesCollectorWrapper<T> {
        FeesCollectorWrapper {
            phantom: PhantomData,
        }
    }
}

impl<T> WeeklyRewardsSplittingTraitsModule for FeesCollectorWrapper<T>
where
    T: FeesCollector,
{
    type WeeklyRewardsSplittingMod = T;

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
