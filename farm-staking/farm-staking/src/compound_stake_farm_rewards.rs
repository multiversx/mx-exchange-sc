use crate::{base_impl_wrapper::FarmStakingWrapper, farm_hooks::hook_type::FarmHookType};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CompoundStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + crate::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
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
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
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
{
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        self.migrate_old_farm_positions(&caller);

        let payments = self.get_non_empty_payments();
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeCompoundRewards,
            caller.clone(),
            payments,
            ManagedVec::new(),
        );

        let mut compound_result = self
            .compound_rewards_base::<FarmStakingWrapper<Self>>(caller.clone(), payments_after_hook);

        let new_farm_token = compound_result.new_farm_token.payment.clone();
        let mut args = ManagedVec::new();
        self.encode_arg_to_vec(&compound_result.compounded_rewards, &mut args);

        let output_payments = self.call_hook(
            FarmHookType::AfterCompoundRewards,
            caller.clone(),
            ManagedVec::from_single_item(new_farm_token),
            args,
        );
        let new_farm_token = output_payments.get(0);
        self.send_payment_non_zero(&caller, &new_farm_token);

        compound_result.new_farm_token.payment = new_farm_token.clone();

        self.set_farm_supply_for_current_week(&compound_result.storage_cache.farm_token_supply);

        self.emit_compound_rewards_event(
            &caller,
            compound_result.context,
            compound_result.new_farm_token,
            compound_result.compounded_rewards,
            compound_result.created_with_merge,
            compound_result.storage_cache,
        );

        new_farm_token
    }
}
