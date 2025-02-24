use contexts::storage_cache::StorageCache;

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
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + energy_query::EnergyQueryModule
    + token_send::TokenSendModule
    + utils::UtilsModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + crate::custom_rewards::CustomRewardsModule
{
    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(&self, opt_user: OptionalValue<ManagedAddress>) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let user = match &opt_user {
            OptionalValue::Some(user) => user,
            OptionalValue::None => &caller,
        };
        if user != &caller {
            require!(
                self.allow_external_claim(user).get(),
                "Cannot claim rewards for this address"
            );
        }

        require!(
            !self.user_total_farm_position(user).is_empty(),
            "User total farm position is empty!"
        );

        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        let boosted_rewards = self.claim_only_boosted_payment(user);
        let reward_nonce = self.reward_nonce().get();
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), reward_nonce, boosted_rewards);

        self.send_payment_non_zero(user, &boosted_rewards_payment);

        boosted_rewards_payment
    }

    // Cannot import the one from farm, as the Wrapper struct has different dependencies
    fn claim_only_boosted_payment(&self, caller: &ManagedAddress) -> BigUint {
        let reward = self.calculate_boosted_rewards(caller);
        if reward > 0 {
            self.reward_reserve().update(|reserve| *reserve -= &reward);
        }

        reward
    }
}
