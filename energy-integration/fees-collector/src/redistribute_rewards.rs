use common_types::{PaymentsVec, TokenAmountPair, Week};
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

const INITIAL_REW_DIST: bool = true;
const INITIAL_REW_NOT_DIST: bool = false;
static INVALID_OFFSET_ERR_MSG: &[u8] = b"Current week must be higher than the week offset";

#[multiversx_sc::module]
pub trait RedistributeRewardsModule:
    crate::fees_accumulation::FeesAccumulationModule
    + crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + crate::additional_locked_tokens::AdditionalLockedTokensModule
{
    #[only_admin]
    #[endpoint(redistributeInitialRewards)]
    fn redistribute_initial_rewards(&self) {
        require!(
            self.redistributed_initial_rewards().get() == INITIAL_REW_NOT_DIST,
            "Initial rewards already distributed"
        );

        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            INVALID_OFFSET_ERR_MSG
        );

        let all_rem_rewards = self.redist_initial_rew(current_week, collect_rewards_offset);
        for reward_entry in &all_rem_rewards {
            self.accumulated_fees(current_week, &reward_entry.token)
                .update(|acc_fees| *acc_fees += reward_entry.amount);
        }

        self.redistributed_initial_rewards().set(INITIAL_REW_DIST);
    }

    #[only_admin]
    #[endpoint(redistributeRewards)]
    fn redistribute_rewards(&self, start_week: Week, end_week: Week) {
        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            INVALID_OFFSET_ERR_MSG
        );
        require!(start_week <= end_week, "Invalid week numbers");
        require!(
            end_week <= current_week - collect_rewards_offset,
            "Invalid end week"
        );

        let all_tokens = self.all_tokens().get();
        let mut all_rewards = ManagedVec::new();
        for token_id in &all_tokens {
            all_rewards.push(TokenAmountPair::new(token_id, BigUint::zero()));
        }

        for week in start_week..=end_week {
            self.accumulate_remaining_rewards_single_week(&mut all_rewards, &all_tokens, week);
        }

        for reward_entry in &all_rewards {
            if reward_entry.amount == 0 {
                continue;
            }

            self.accumulated_fees(current_week, &reward_entry.token)
                .update(|acc_fees| *acc_fees += reward_entry.amount);
        }
    }

    fn accumulate_remaining_rewards_single_week(
        &self,
        all_rewards: &mut ManagedVec<TokenAmountPair<Self::Api>>,
        all_tokens: &ManagedVec<TokenIdentifier>,
        week: Week,
    ) {
        let remaining_rewards = self.remaining_rewards(week).take();
        for rem_rew_entry in &remaining_rewards {
            if rem_rew_entry.amount == 0 {
                continue;
            }

            let opt_index = all_tokens.find(&rem_rew_entry.token_identifier);
            if opt_index.is_none() {
                continue;
            }

            let index = unsafe { opt_index.unwrap_unchecked() };
            let mut rew_entry = all_rewards.get_mut(index);
            rew_entry.amount += rem_rew_entry.amount;
        }
    }

    fn redist_initial_rew(
        &self,
        current_week: Week,
        collect_rewards_offset: Week,
    ) -> ManagedVec<TokenAmountPair<Self::Api>> {
        let all_tokens = self.all_tokens().get();
        let locked_token_id = self.locked_token_id().get();
        let start_week = current_week - collect_rewards_offset;
        let end_week = current_week;

        let mut all_rem_rewards = ManagedVec::new();
        for token_id in &all_tokens {
            if token_id == locked_token_id {
                continue;
            }

            let mut token_balance = self
                .blockchain()
                .get_sc_balance(&EgldOrEsdtTokenIdentifier::esdt(token_id.clone()), 0);
            if token_balance == 0 {
                continue;
            }

            for week in start_week..=end_week {
                let to_dist_week = self.accumulated_fees(week, &token_id).get();
                token_balance -= to_dist_week;
            }

            if token_balance == 0 {
                continue;
            }

            all_rem_rewards.push(TokenAmountPair::new(token_id, token_balance));
        }

        all_rem_rewards
    }

    #[view(wereInitialRewardsRedistributed)]
    #[storage_mapper("redistributedInitialRewards")]
    fn redistributed_initial_rewards(&self) -> SingleValueMapper<bool>;

    #[view(getRemainingRewards)]
    #[storage_mapper("remainingRewards")]
    fn remaining_rewards(&self, week: Week) -> SingleValueMapper<PaymentsVec<Self::Api>>;
}
