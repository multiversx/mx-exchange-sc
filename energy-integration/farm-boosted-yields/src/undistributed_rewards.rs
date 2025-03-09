multiversx_sc::imports!();

use common_types::Week;
use energy_factory::unlocked_token_transfer::ProxyTrait as _;
use week_timekeeping::FIRST_WEEK;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

pub const MIN_GAS_FOR_PROCESS: u64 = 5_000_000;

#[multiversx_sc::module]
pub trait UndistributedRewardsModule:
    config::ConfigModule
    + week_timekeeping::WeekTimekeepingModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + energy_query::EnergyQueryModule
{
    #[only_owner]
    #[endpoint(collectUndistributedBoostedRewards)]
    fn collect_undistributed_boosted_rewards(&self) -> BigUint {
        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            "Current week must be higher than the week offset"
        );

        let last_collect_week = self.last_collect_undist_week().get();
        let start_week = if last_collect_week > 0 {
            last_collect_week
        } else {
            FIRST_WEEK
        };
        let mut end_week = current_week - collect_rewards_offset;

        let mut total_rewards = BigUint::zero();
        let min_gas_for_process = self.min_gas_for_process().get();
        for week in start_week..=end_week {
            let rewards_to_distribute = self.remaining_boosted_rewards_to_distribute(week).take();
            total_rewards += rewards_to_distribute;

            let remaining_gas = self.blockchain().get_gas_left();
            if remaining_gas <= min_gas_for_process {
                end_week = week;
                break;
            }
        }

        self.last_collect_undist_week().set(end_week + 1);

        if total_rewards == 0 {
            return total_rewards;
        }

        self.distribute_leftover_rewards(&total_rewards);

        total_rewards
    }

    fn distribute_leftover_rewards(&self, total_rewards: &BigUint) {
        let base_token_id = self.get_base_token_id();
        let reward_token_id = self.reward_token_id().get();
        if base_token_id == reward_token_id {
            let energy_factory = self.energy_factory_address().get();
            let _: () = self
                .energy_factory_proxy(energy_factory)
                .transfer_unlocked_token(total_rewards.clone())
                .execute_on_dest_context();
        } else {
            let owner = self.blockchain().get_caller();
            self.send()
                .direct_esdt(&owner, &reward_token_id, 0, total_rewards);
        }
    }

    #[only_owner]
    #[endpoint(setMinGasForProcess)]
    fn set_min_gas_for_process(&self, min_gas_for_process: u64) {
        self.min_gas_for_process().set(min_gas_for_process);
    }

    #[view(getRemainingBoostedRewardsToDistribute)]
    #[storage_mapper("remainingBoostedRewardsToDistribute")]
    fn remaining_boosted_rewards_to_distribute(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("multisigAddress")]
    fn multisig_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("minGasForProcess")]
    fn min_gas_for_process(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("lastCollectUndistWeek")]
    fn last_collect_undist_week(&self) -> SingleValueMapper<Week>;
}
