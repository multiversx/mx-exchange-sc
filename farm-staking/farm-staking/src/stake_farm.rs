multiversx_sc::imports!();

use common_structs::PaymentsVec;
use farm::EnterFarmResultType;

use crate::base_impl_wrapper::FarmStakingWrapper;

#[multiversx_sc::module]
pub trait StakeFarmModule:
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
{
    #[payable("*")]
    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(
        &self,
        staked_token_amount: BigUint,
        original_caller: ManagedAddress,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let staked_token_id = self.farming_token_id().get();
        let staked_token_simulated_payment =
            EsdtTokenPayment::new(staked_token_id, 0, staked_token_amount);

        let farm_tokens = self.call_value().all_esdt_transfers().clone_value();
        let mut payments = ManagedVec::from_single_item(staked_token_simulated_payment);
        payments.append_vec(farm_tokens);

        self.stake_farm_common(original_caller, payments)
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);
        let payments = self.get_non_empty_payments();

        self.stake_farm_common(original_caller, payments)
    }

    fn stake_farm_common(
        &self,
        original_caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> EnterFarmResultType<Self::Api> {
        let first_additional_payment_index = 1;
        let boosted_rewards = match payments.try_get(first_additional_payment_index) {
            Some(p) => self.claim_only_boosted_payment(&original_caller, &p),
            None => EsdtTokenPayment::new(self.reward_token_id().get(), 0, BigUint::zero()),
        };

        let enter_result =
            self.enter_farm_base::<FarmStakingWrapper<Self>>(original_caller, payments);

        let caller = self.blockchain().get_caller();
        let new_farm_token = enter_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards);

        self.set_farm_supply_for_current_week(&enter_result.storage_cache.farm_token_supply);

        self.emit_enter_farm_event(
            &caller,
            enter_result.context.farming_token_payment,
            enter_result.new_farm_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        (new_farm_token, boosted_rewards).into()
    }
}
