#![no_std]

elrond_wasm::imports!();

use common_errors::ERROR_ENERGY_UPDATE_SAME_WEEK;
use common_types::{PaymentsVec, Week};
use core::marker::PhantomData;
use energy_query::Energy;
use weekly_rewards_splitting::{base_impl::WeeklyRewardsSplittingTraitsModule, ClaimProgress};

use elrond_wasm_modules::ongoing_operation::{
    CONTINUE_OP, DEFAULT_MIN_GAS_TO_SAVE_PROGRESS, STOP_OP,
};

pub mod config;
pub mod events;
pub mod fees_accumulation;

#[elrond_wasm::contract]
pub trait FeesCollector:
    config::ConfigModule
    + events::FeesCollectorEventsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + fees_accumulation::FeesAccumulationModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + elrond_wasm_modules::pause::PauseModule
    + elrond_wasm_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(&self) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> PaymentsVec<Self::Api> {
        require!(self.not_paused(), "Cannot claim while paused");
        let caller = self.blockchain().get_caller();
        let wrapper = FeesCollectorWrapper::new();
        let rewards = self.claim_multi(&wrapper, &caller);
        if !rewards.is_empty() {
            self.send().direct_multi(&caller, &rewards);
        }

        rewards
    }

    /// Accepts pairs of (user address, energy amount, total locked tokens).
    /// Sets the given amounts for the user's positions,
    /// and recomputes the global amounts.
    ///
    /// Returns whether the operation was fully completed.
    /// If not, it also returns the last processed index.
    #[only_owner]
    #[endpoint(recomputeEnergy)]
    fn recompute_energy(
        &self,
        arg_pairs: MultiValueEncoded<MultiValue3<ManagedAddress, BigUint, BigUint>>,
    ) -> MultiValue2<OperationCompletionStatus, OptionalValue<usize>> {
        require!(self.is_paused(), "May only recompute while paused");

        let current_week = self.get_current_week();
        let current_epoch = self.blockchain().get_block_epoch();

        let mut iter = arg_pairs.into_iter().enumerate();
        let mut last_processed_index = 0;

        let run_result = self.run_while_it_has_gas(DEFAULT_MIN_GAS_TO_SAVE_PROGRESS, || match iter
            .next()
        {
            Some((index, multi_value)) => {
                let (user, energy, total_locked) = multi_value.into_tuple();
                let energy_entry = Energy::new(BigInt::from(energy), current_epoch, total_locked);
                self.update_user_energy_for_current_week(&user, current_week, &energy_entry);

                self.current_claim_progress(&user).update(|claim_progress| {
                    if claim_progress.week == current_week {
                        claim_progress.energy = energy_entry;
                    }
                });

                last_processed_index = index;

                CONTINUE_OP
            }
            None => STOP_OP,
        });

        match run_result {
            OperationCompletionStatus::Completed => (run_result, OptionalValue::None).into(),
            OperationCompletionStatus::InterruptedBeforeOutOfGas => {
                (run_result, OptionalValue::Some(last_processed_index)).into()
            }
        }
    }

    #[endpoint(updateEnergyForUser)]
    fn update_energy_for_user(&self, user: ManagedAddress) {
        let current_week = self.get_current_week();

        let claim_progress_mapper = self.current_claim_progress(&user);

        let is_new_user = claim_progress_mapper.is_empty();
        let claim_progress = if is_new_user {
            let new_claim_progress = ClaimProgress {
                energy: Energy::new_zero_energy(0),
                week: current_week,
            };
            claim_progress_mapper.set(&new_claim_progress);
            new_claim_progress
        } else {
            claim_progress_mapper.get()
        };

        
        require!(
            claim_progress.week == current_week,
            ERROR_ENERGY_UPDATE_SAME_WEEK
        );
        self.update_energy_and_progress_after_enter(&user);
    }

    fn update_energy_and_progress_after_enter(&self, caller: &ManagedAddress) {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(caller);
        self.update_user_energy_for_current_week(caller, current_week, &current_user_energy);
        self.current_claim_progress(caller).set(ClaimProgress {
            energy: current_user_energy,
            week: current_week,
        });
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
        module: &Self::WeeklyRewardsSplittingMod,
        week: Week,
    ) -> PaymentsVec<<Self::WeeklyRewardsSplittingMod as ContractBase>::Api> {
        let mut results = ManagedVec::new();
        let all_tokens = module.all_tokens().get();
        for token in &all_tokens {
            let opt_accumulated_fees = module.get_and_clear_acccumulated_fees(week, &token);
            if let Some(accumulated_fees) = opt_accumulated_fees {
                results.push(EsdtTokenPayment::new(token, 0, accumulated_fees));
            }
        }

        results
    }
}
