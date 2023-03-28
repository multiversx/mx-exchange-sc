multiversx_sc::imports!();

use farm::base_functions::ClaimRewardsResultType;

use crate::base_impl_wrapper::FarmStakingWrapper;

#[multiversx_sc::module]
pub trait ClaimStakeFarmRewardsModule:
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
{
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);

        self.claim_rewards_common(original_caller, None)
    }

    #[payable("*")]
    #[endpoint(claimRewardsWithNewValue)]
    fn claim_rewards_with_new_value(
        &self,
        new_farming_amount: BigUint,
        original_caller: ManagedAddress,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        self.claim_rewards_common(original_caller, Some(new_farming_amount))
    }

    fn claim_rewards_common(
        &self,
        original_caller: ManagedAddress,
        opt_new_farming_amount: Option<BigUint>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
        let mut claim_result = self
            .claim_rewards_base_no_farm_token_mint::<FarmStakingWrapper<Self>>(
                original_caller,
                ManagedVec::from_single_item(payment),
            );

        let mut virtual_farm_token = claim_result.new_farm_token.clone();
        if let Some(new_amount) = opt_new_farming_amount {
            claim_result.storage_cache.farm_token_supply -= &virtual_farm_token.payment.amount;
            claim_result.storage_cache.farm_token_supply += &new_amount;

            virtual_farm_token.payment.amount = new_amount.clone();
            virtual_farm_token.attributes.current_farm_amount = new_amount;

            self.set_farm_supply_for_current_week(&claim_result.storage_cache.farm_token_supply);
        }

        let new_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token.payment.token_identifier,
            &virtual_farm_token.payment.amount,
            &virtual_farm_token.attributes,
        );
        virtual_farm_token.payment.token_nonce = new_farm_token_nonce;

        let caller = self.blockchain().get_caller();
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
