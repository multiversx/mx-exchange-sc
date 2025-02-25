use common_types::{PaymentsVec, Week};
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ClaimUndistRewardsModule:
    crate::fees_accumulation::FeesAccumulationModule
    + crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    #[only_admin]
    #[endpoint(setMultisigAddress)]
    fn set_multisig_address(&self, ms_address: ManagedAddress) {
        self.require_sc_address(&ms_address);

        self.multisig_address().set(ms_address);
    }

    #[only_admin]
    #[endpoint(claimUndistributedRewards)]
    fn claim_undistributed_rewards(&self, start_week: Week, end_week: Week) -> BigUint {
        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            "Current week must be higher than the week offset"
        );
        require!(start_week <= end_week, "Invalid week numbers");
        require!(
            end_week <= current_week - collect_rewards_offset,
            "Invalid end week"
        );

        let locked_token_id = self.locked_token_id().get();
        let mut total_locked_token_rewards = BigUint::zero();
        for week in start_week..=end_week {
            let locked_token_rewards =
                self.accumulate_remaining_locked_rewards_single_week(&locked_token_id, week);
            total_locked_token_rewards += locked_token_rewards;
        }

        if total_locked_token_rewards == 0 {
            return total_locked_token_rewards;
        }

        let base_token_id = self.get_base_token_id();
        self.send()
            .esdt_local_mint(&base_token_id, 0, &total_locked_token_rewards);

        let ms_address = self.multisig_address().get();
        self.send()
            .direct_esdt(&ms_address, &base_token_id, 0, &total_locked_token_rewards);

        total_locked_token_rewards
    }

    fn accumulate_remaining_locked_rewards_single_week(
        &self,
        locked_token_id: &TokenIdentifier,
        week: Week,
    ) -> BigUint {
        let remaining_rewards_mapper = self.remaining_rewards(week);
        let mut remaining_rewards = remaining_rewards_mapper.get();
        for (i, rem_rew_entry) in remaining_rewards.iter().enumerate() {
            if &rem_rew_entry.token_identifier != locked_token_id {
                continue;
            }

            remaining_rewards.remove(i);
            remaining_rewards_mapper.set(remaining_rewards);

            return rem_rew_entry.amount;
        }

        BigUint::zero()
    }

    #[storage_mapper("msAddress")]
    fn multisig_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getRemainingRewards)]
    #[storage_mapper("remainingRewards")]
    fn remaining_rewards(&self, week: Week) -> SingleValueMapper<PaymentsVec<Self::Api>>;
}
