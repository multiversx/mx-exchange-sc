use common_types::Week;
use week_timekeeping::FIRST_WEEK;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

mod energy_factory_proxy_send_rew {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait EnergyFactorySendRewProxy {
        #[endpoint(transferUnlockedToken)]
        fn transfer_unlocked_token(&self, dest: ManagedAddress, amount: BigUint);
    }
}

#[multiversx_sc::module]
pub trait UndistributedRewardsModule:
    config::ConfigModule
    + week_timekeeping::WeekTimekeepingModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + energy_query::EnergyQueryModule
{
    #[only_owner]
    #[endpoint(setMultisigAddress)]
    fn set_multisig_address(&self, multisig_address: ManagedAddress) {
        self.multisig_address().set(multisig_address);
    }

    #[only_owner]
    #[endpoint(collectUndistributedBoostedRewards)]
    fn collect_undistributed_boosted_rewards(&self) -> BigUint {
        require!(
            !self.multisig_address().is_empty(),
            "No multisig address set"
        );

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
        let end_week = current_week - collect_rewards_offset;

        let mut total_rewards = BigUint::zero();
        for week in start_week..=end_week {
            let rewards_to_distribute = self.remaining_boosted_rewards_to_distribute(week).take();
            total_rewards += rewards_to_distribute;
        }

        self.last_collect_undist_week().set(end_week + 1);

        if total_rewards == 0 {
            return total_rewards;
        }

        self.distribute_leftover_rewards(&total_rewards);

        total_rewards
    }

    fn distribute_leftover_rewards(&self, total_rewards: &BigUint) {
        let multisig_address = self.multisig_address().get();
        let base_token_id = self.get_base_token_id();
        let reward_token_id = self.reward_token_id().get();
        if base_token_id == reward_token_id {
            let energy_factory = self.energy_factory_address().get();
            let _: () = self
                .energy_factory_send_rew_proxy_obj(energy_factory)
                .transfer_unlocked_token(multisig_address, total_rewards.clone())
                .execute_on_dest_context();
        } else {
            self.send()
                .direct_esdt(&multisig_address, &reward_token_id, 0, total_rewards);
        }
    }

    #[view(getRemainingBoostedRewardsToDistribute)]
    #[storage_mapper("remainingBoostedRewardsToDistribute")]
    fn remaining_boosted_rewards_to_distribute(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("multisigAddress")]
    fn multisig_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("lastCollectUndistWeek")]
    fn last_collect_undist_week(&self) -> SingleValueMapper<Week>;

    #[proxy]
    fn energy_factory_send_rew_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> energy_factory_proxy_send_rew::Proxy<Self::Api>;
}
