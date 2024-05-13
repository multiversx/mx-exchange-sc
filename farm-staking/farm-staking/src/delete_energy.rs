use common_structs::{FarmToken, PaymentsVec};
use farm_base_impl::base_traits_impl::FarmContract;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait DeleteEnergyModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
{
    fn delete_user_energy_if_needed<FC: FarmContract<FarmSc = Self>>(
        &self,
        payments: &PaymentsVec<Self::Api>,
        all_attributes: &ManagedVec<FC::AttributesType>,
    ) {
        let mut processed_users = ManagedMap::new();
        for (payment, attr) in payments.iter().zip(all_attributes.into_iter()) {
            let original_owner = attr.get_original_owner();
            if processed_users.contains(original_owner.as_managed_buffer())
                || self.is_old_farm_position(payment.token_nonce)
            {
                continue;
            }

            self.clear_user_energy_if_needed(&original_owner);

            processed_users.put(original_owner.as_managed_buffer(), &ManagedBuffer::new());
        }
    }
}
