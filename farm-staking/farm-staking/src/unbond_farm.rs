multiversx_sc::imports!();

use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::{base_impl_wrapper::FarmStakingWrapper, token_attributes::UnbondSftAttributes};

#[multiversx_sc::module]
pub trait UnbondFarmModule:
    crate::custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
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
    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(&self) -> EsdtTokenPayment {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let farm_token_mapper = self.farm_token();
        let payment = self.call_value().single_esdt();
        farm_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: UnbondSftAttributes =
            farm_token_mapper.get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Unbond period not over"
        );

        let current_week = self.get_current_week();
        self.perform_weekly_update(current_week);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);
        self.set_farm_supply_for_current_week(&storage_cache.farm_token_supply);

        farm_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let caller = self.blockchain().get_caller();
        let farming_tokens =
            EsdtTokenPayment::new(storage_cache.farming_token_id.clone(), 0, payment.amount);
        self.send_payment_non_zero(&caller, &farming_tokens);

        farming_tokens
    }
}
