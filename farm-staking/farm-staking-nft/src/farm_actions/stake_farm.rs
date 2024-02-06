multiversx_sc::imports!();

use farm::EnterFarmResultType;

use crate::{base_impl_wrapper::FarmStakingWrapper, farm_hooks::hook_type::FarmHookType};

#[multiversx_sc::module]
pub trait StakeFarmModule:
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
    + farm_base_impl::enter_farm::BaseEnterFarmModule
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
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(&self) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeStake,
            caller.clone(),
            payments,
            ManagedVec::new(),
        );

        let boosted_rewards = self.claim_only_boosted_payment(&caller);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let mut enter_result =
            self.enter_farm_base::<FarmStakingWrapper<Self>>(caller.clone(), payments_after_hook);

        let new_farm_token = enter_result.new_farm_token.payment.clone();
        let mut output_payments = ManagedVec::new();
        output_payments.push(new_farm_token);
        self.push_if_non_zero_payment(&mut output_payments, boosted_rewards_payment.clone());

        let mut output_payments_after_hook = self.call_hook(
            FarmHookType::AfterStake,
            caller.clone(),
            output_payments,
            ManagedVec::new(),
        );
        let new_farm_token = self.pop_first_payment(&mut output_payments_after_hook);
        let boosted_rewards_payment =
            self.pop_or_return_payment(&mut output_payments_after_hook, boosted_rewards_payment);

        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);

        self.set_farm_supply_for_current_week(&enter_result.storage_cache.farm_token_supply);

        self.update_energy_and_progress(&caller);

        enter_result.new_farm_token.payment = new_farm_token.clone();

        self.emit_enter_farm_event(
            &caller,
            enter_result.context.farming_token_payment,
            enter_result.new_farm_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        (new_farm_token, boosted_rewards_payment).into()
    }
}
