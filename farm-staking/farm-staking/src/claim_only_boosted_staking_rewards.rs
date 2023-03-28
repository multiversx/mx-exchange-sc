use crate::base_impl_wrapper::FarmStakingWrapper;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ClaimOnlyBoostedStakingRewardsModule:
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
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + crate::custom_rewards::CustomRewardsModule
{
    // Cannot import the one from farm, as the Wrapper struct has different dependencies
    fn claim_only_boosted_payment(
        &self,
        caller: &ManagedAddress,
        payment: &EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let farm_token_mapper = self.farm_token();
        farm_token_mapper.require_same_token(&payment.token_identifier);

        let token_attributes =
            self.get_attributes_as_part_of_fixed_supply(payment, &farm_token_mapper);
        let reward = FarmStakingWrapper::<Self>::calculate_boosted_rewards(
            self,
            caller,
            &token_attributes,
            payment.amount.clone(),
        );
        if reward > 0 {
            self.reward_reserve().update(|reserve| *reserve -= &reward);
        }

        let reward_token_id = self.reward_token_id().get();
        EsdtTokenPayment::new(reward_token_id, 0, reward)
    }
}
