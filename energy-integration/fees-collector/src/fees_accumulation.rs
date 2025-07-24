multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use energy_factory::lock_options::MAX_PENALTY_PERCENTAGE;
use week_timekeeping::Week;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

#[multiversx_sc::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
{
    /// Anyone can deposit tokens through this endpoint
    ///
    /// Deposits for current week are accessible starting next week
    ///
    /// The contract accepts all payments but only the base and locked tokens are verified and allocated
    #[payable("*")]
    #[endpoint(depositSwapFees)]
    fn deposit_swap_fees(&self) {
        let caller = self.blockchain().get_caller();
        let mut payment = self.call_value().single_esdt();

        let current_week = self.get_current_week();
        let base_token_id = self.get_base_token_id();

        if payment.token_nonce != 0 {
            require!(
                !self.blockchain().is_smart_contract(&caller)
                    || self.known_contracts().contains(&caller),
                "Caller must be a known contract"
            );

            self.try_burn_locked_token(&payment);

            self.accumulated_fees(current_week, &payment.token_identifier)
                .update(|amt| *amt += &payment.amount);
        } else if payment.token_identifier == base_token_id {
            self.burn_part_of_base_token(&mut payment);

            self.accumulated_fees(current_week, &payment.token_identifier)
                .update(|amt| *amt += &payment.amount);
        }

        self.emit_deposit_swap_fees_event(&caller, current_week, &payment);
    }

    fn try_burn_locked_token(&self, payment: &EsdtTokenPayment) {
        let locked_token_id = self.get_locked_token_id();
        require!(
            payment.token_identifier == locked_token_id,
            "Only locked token accepted as SFT/NFT/MetaESDT"
        );

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );
    }

    fn burn_part_of_base_token(&self, payment: &mut EsdtTokenPayment) {
        let burn_percent = self.base_token_burn_percent().get();
        if burn_percent == 0 {
            return;
        }

        let burn_amount = &payment.amount * burn_percent / MAX_PENALTY_PERCENTAGE;
        if burn_amount == 0 {
            return;
        }

        self.send()
            .esdt_local_burn(&payment.token_identifier, 0, &burn_amount);

        payment.amount -= burn_amount;
    }

    fn get_and_clear_accumulated_fees(
        &self,
        week: Week,
        token: &TokenIdentifier,
    ) -> Option<BigUint> {
        let value = self.accumulated_fees(week, token).take();
        if value > 0 {
            Some(value)
        } else {
            None
        }
    }

    fn get_token_available_amount(
        &self,
        current_week: Week,
        token_id: &TokenIdentifier,
    ) -> BigUint {
        let (start_week, end_week) = self.get_week_range(current_week);
        let remaining_claimable_token_amount =
            self.calculate_remaining_claimable_token_amount(start_week, end_week, token_id);

        self.calculate_available_balance(token_id, remaining_claimable_token_amount)
    }

    fn get_week_range(&self, current_week: Week) -> (Week, Week) {
        let start_week = if current_week >= USER_MAX_CLAIM_WEEKS {
            current_week - USER_MAX_CLAIM_WEEKS
        } else {
            0
        };
        (start_week, current_week)
    }

    fn calculate_remaining_claimable_token_amount(
        &self,
        start_week: Week,
        end_week: Week,
        token_id: &TokenIdentifier,
    ) -> BigUint {
        let mut remaining_claimable_token_amount = BigUint::zero();

        for week in start_week..=end_week {
            let mut week_amount = self.accumulated_fees(week, token_id).get();
            week_amount += self.find_total_reward_amount_for_token(week, token_id);
            week_amount -= self.rewards_claimed(week, token_id).get();

            remaining_claimable_token_amount += week_amount;
        }

        remaining_claimable_token_amount
    }

    fn find_total_reward_amount_for_token(
        &self,
        week: Week,
        token_id: &TokenIdentifier,
    ) -> BigUint {
        let total_rewards_for_week = self.total_rewards_for_week(week).get();

        for reward in &total_rewards_for_week {
            if &reward.token_identifier == token_id {
                return reward.amount.clone();
            }
        }

        BigUint::zero()
    }

    fn calculate_available_balance(
        &self,
        token_id: &TokenIdentifier,
        remaining_claimable_token_amount: BigUint,
    ) -> BigUint {
        let sc_address = self.blockchain().get_sc_address();
        let token_total_balance = self.blockchain().get_esdt_balance(&sc_address, token_id, 0);

        if token_total_balance <= remaining_claimable_token_amount {
            return BigUint::zero();
        }

        token_total_balance - remaining_claimable_token_amount
    }
}
