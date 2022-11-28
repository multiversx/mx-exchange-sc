#![no_std]

elrond_wasm::imports!();

use common_types::{PaymentsVec, Week};
use core::marker::PhantomData;
use weekly_rewards_splitting::base_impl::WeeklyRewardsSplittingTraitsModule;

pub mod additional_locked_tokens;
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
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + fees_accumulation::FeesAccumulationModule
    + additional_locked_tokens::AdditionalLockedTokensModule
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

        let mut tokens = MultiValueEncoded::new();
        tokens.push(locked_token_id.clone());
        self.add_known_tokens(tokens);

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
        let mut output_rewards = ManagedVec::new();
        if rewards.is_empty() {
            return output_rewards;
        }

        let locked_token_id = self.locked_token_id().get();
        let mut opt_locked_rewards = None;
        for reward in &rewards {
            if reward.token_identifier == locked_token_id {
                let energy_factory_addr = self.energy_factory_address().get();
                let locked_rewards = self.lock_virtual(
                    self.get_base_token_id(&energy_factory_addr),
                    reward.amount,
                    caller.clone(),
                    caller.clone(),
                );
                opt_locked_rewards = Some(locked_rewards);
            } else {
                output_rewards.push(EsdtTokenPayment::new(
                    reward.token_identifier,
                    reward.token_nonce,
                    reward.amount,
                ));
            }
        }

        if !output_rewards.is_empty() {
            self.send().direct_multi(&caller, &output_rewards);
        }

        if let Some(locked_rewards) = opt_locked_rewards {
            output_rewards.push(locked_rewards);
        }

        output_rewards
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
