multiversx_sc::imports!();

use contexts::storage_cache::StorageCache;

use crate::common::{result_types::UnbondResultType, token_attributes::UnbondSftAttributes};

#[multiversx_sc::module]
pub trait UnbondFarmModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
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
    + crate::common::unbond_token::UnbondTokenModule
{
    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(&self) -> UnbondResultType<Self::Api> {
        let storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let unbond_token_mapper = self.unbond_token();
        let payment = self.call_value().single_esdt();
        unbond_token_mapper.require_same_token(&payment.token_identifier);

        let caller = self.blockchain().get_caller();
        let attributes: UnbondSftAttributes<Self::Api> =
            unbond_token_mapper.get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Unbond period not over"
        );

        unbond_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let farming_tokens = attributes.farming_token_parts;
        self.send_multiple_tokens_if_not_zero(&caller, &farming_tokens);

        UnbondResultType { farming_tokens }
    }
}
