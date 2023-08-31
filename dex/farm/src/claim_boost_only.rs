multiversx_sc::imports!();

use crate::base_functions::Wrapper;

#[multiversx_sc::module]
pub trait ClaimBoostOnlyModule:
    config::ConfigModule
    + rewards::RewardsModule
    + farm_token::FarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + week_timekeeping::WeekTimekeepingModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + token_send::TokenSendModule
    + events::EventsModule
    + crate::exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + crate::base_functions::BaseFunctionsModule
{
    #[payable("*")]
    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(
        &self,
        opt_user: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let user = match opt_user {
            OptionalValue::Some(user) => user,
            #[allow(clippy::redundant_clone)]
            OptionalValue::None => caller.clone(),
        };
        let user_total_farm_position_struct = self.get_user_total_farm_position_struct(&user);
        if user != caller {
            require!(
                user_total_farm_position_struct.allow_external_claim_boosted_rewards,
                "Cannot claim rewards for this address"
            );
        }

        let reward_token_id = self.reward_token_id().get();
        let user_total_farm_position = user_total_farm_position_struct.total_farm_position;
        if user_total_farm_position == BigUint::zero() {
            return EsdtTokenPayment::new(reward_token_id, 0, BigUint::zero());
        }

        let reward = self.claim_boosted_yields_rewards(&user, user_total_farm_position);
        if reward > 0 {
            self.reward_reserve().update(|reserve| *reserve -= &reward);
        }

        let boosted_rewards = EsdtTokenPayment::new(reward_token_id, 0, reward);
        self.send_payment_non_zero(&user, &boosted_rewards);

        self.update_energy_and_progress(&user);

        boosted_rewards
    }

    fn claim_only_boosted_payment(
        &self,
        caller: &ManagedAddress,
        payment: &EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let farm_token_mapper = self.farm_token();
        farm_token_mapper.require_same_token(&payment.token_identifier);

        let token_attributes =
            self.get_attributes_as_part_of_fixed_supply(payment, &farm_token_mapper);
        let reward = Wrapper::<Self>::calculate_boosted_rewards(self, caller, &token_attributes);
        if reward > 0 {
            self.reward_reserve().update(|reserve| *reserve -= &reward);
        }

        let reward_token_id = self.reward_token_id().get();
        EsdtTokenPayment::new(reward_token_id, 0, reward)
    }
}
