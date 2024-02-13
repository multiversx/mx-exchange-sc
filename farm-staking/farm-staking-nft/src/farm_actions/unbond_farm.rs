multiversx_sc::imports!();

use contexts::storage_cache::StorageCache;

use crate::{
    common::{result_types::UnbondResultType, token_attributes::UnbondSftAttributes},
    farm_hooks::hook_type::FarmHookType,
};

#[multiversx_sc::module]
pub trait UnbondFarmModule:
    rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
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
    + banned_addresses::BannedAddressModule
    + crate::farm_hooks::change_hooks::ChangeHooksModule
    + crate::farm_hooks::call_hook::CallHookModule
    + crate::unbond_token::UnbondTokenModule
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
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeUnbond,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            ManagedVec::new(),
        );
        let payment = payments_after_hook.get(0);

        let attributes: UnbondSftAttributes<Self::Api> =
            unbond_token_mapper.get_token_attributes(payment.token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Unbond period not over"
        );

        unbond_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let farming_tokens = attributes.farming_token_parts;
        let output_payments_after_hook = self.call_hook(
            FarmHookType::AfterUnbond,
            caller.clone(),
            farming_tokens,
            ManagedVec::new(),
        );

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments_after_hook);

        UnbondResultType {
            farming_tokens: output_payments_after_hook,
        }
    }
}
