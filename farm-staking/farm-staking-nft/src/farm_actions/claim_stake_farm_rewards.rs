multiversx_sc::imports!();

use farm::base_functions::ClaimRewardsResultType;

use crate::{base_impl_wrapper::FarmStakingNftWrapper, farm_hooks::hook_type::FarmHookType};

#[multiversx_sc::module]
pub trait ClaimStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
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
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeClaimRewards,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            ManagedVec::new(),
        );
        let payment = payments_after_hook.get(0);

        let mut claim_result = self.claim_rewards_base::<FarmStakingNftWrapper<Self>>(
            caller.clone(),
            ManagedVec::from_single_item(payment),
        );

        let mut virtual_farm_token = claim_result.new_farm_token.clone();

        self.update_energy_and_progress(&caller);

        let mut output_payments = ManagedVec::new();
        output_payments.push(virtual_farm_token.payment);
        self.push_if_non_zero_payment(&mut output_payments, claim_result.rewards.clone());

        // TODO: Fix attributes
        let mut output_payments_after_hook = self.call_hook(
            FarmHookType::AfterClaimRewards,
            caller.clone(),
            output_payments,
            ManagedVec::new(),
        );
        virtual_farm_token.payment = self.pop_first_payment(&mut output_payments_after_hook);
        claim_result.rewards =
            self.pop_or_return_payment(&mut output_payments_after_hook, claim_result.rewards);

        self.send_payment_non_zero(&caller, &virtual_farm_token.payment);
        self.send_payment_non_zero(&caller, &claim_result.rewards);

        self.emit_claim_rewards_event(
            &caller,
            claim_result.context,
            virtual_farm_token.clone(),
            claim_result.rewards.clone(),
            claim_result.created_with_merge,
            claim_result.storage_cache,
        );

        (virtual_farm_token.payment, claim_result.rewards).into()
    }
}
