#![no_std]

elrond_wasm::imports!();

use common_types::{PaymentsVec, Week};
use core::marker::PhantomData;
use weekly_rewards_splitting::base_impl::WeeklyRewardsSplittingTraitsModule;

pub mod additional_locked_tokens;
pub mod config;
pub mod events;
pub mod fees_accumulation;
pub mod payments_grouping;

#[elrond_wasm::contract]
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
    + payments_grouping::PaymentsGroupingModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + elrond_wasm_modules::pause::PauseModule
    + utils::UtilsModule
{
    #[init]
    fn init(&self, locked_token_id: TokenIdentifier, energy_factory_address: ManagedAddress) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);
        self.require_valid_token_id(&locked_token_id);
        self.require_sc_address(&energy_factory_address);

        self.locked_token_id().set(locked_token_id);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> PaymentsVec<Self::Api> {
        require!(self.not_paused(), "Cannot claim while paused");

        self.accumulate_additional_locked_tokens();

        let caller = self.blockchain().get_caller();
        let wrapper = FeesCollectorWrapper::new();
        let rewards = self.claim_multi(&wrapper, &caller);
        let mut merged_rewards = self.group_payments(rewards);
        if merged_rewards.is_empty() {
            return merged_rewards;
        }

        let opt_locked_rewards = self.get_and_remove_locked_token_rewards(&mut merged_rewards);
        let opt_minted_locked_rewards = match opt_locked_rewards {
            Some(locked_rewards) => {
                if locked_rewards.amount > 0 {
                    let energy_factory_addr = self.energy_factory_address().get();
                    let new_locked_tokens = self.lock_virtual(
                        self.get_base_token_id(&energy_factory_addr),
                        locked_rewards.amount,
                        caller.clone(),
                        caller.clone(),
                    );

                    Some(new_locked_tokens)
                } else {
                    None
                }
            }
            None => None,
        };

        if !merged_rewards.is_empty() {
            self.send().direct_multi(&caller, &merged_rewards);
        }

        if let Some(minted_rewards) = opt_minted_locked_rewards {
            merged_rewards.push(minted_rewards);
        }

        merged_rewards
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
        let empty_buffer = ManagedBuffer::new();
        for token in &all_tokens {
            let accumulated_fees = if token.as_managed_buffer() != &empty_buffer {
                sc.get_and_clear_acccumulated_fees(week, &token)
            } else {
                BigUint::zero()
            };

            results.push(EsdtTokenPayment::new(token, 0, accumulated_fees));
        }

        let locked_token_id = sc.locked_token_id().get();
        let locked_token_rewards = sc.get_and_clear_acccumulated_fees(week, &locked_token_id);
        results.push(EsdtTokenPayment::new(
            locked_token_id,
            0,
            locked_token_rewards,
        ));

        results
    }
}
