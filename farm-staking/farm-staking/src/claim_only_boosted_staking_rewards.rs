use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::base_impl_wrapper::FarmStakingWrapper;

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
    + events::EventsModule
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
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let boosted_rewards = self.claim_only_boosted_payment(user);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        self.set_farm_supply_for_current_week(&storage_cache.farm_token_supply);

        self.send_payment_non_zero(user, &boosted_rewards_payment);

        boosted_rewards_payment
    }

    fn migrate_old_farm_positions(&self, caller: &ManagedAddress) -> BigUint {
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let farm_token_mapper = self.farm_token();
        let farm_token_id = farm_token_mapper.get_token_id();
        let mut migrated_amount = BigUint::zero();
        for farm_position in &payments {
            if farm_position.token_identifier == farm_token_id
                && self.is_old_farm_position(farm_position.token_nonce)
            {
                migrated_amount += farm_position.amount;
            }
        }

        if migrated_amount > 0 {
            self.user_total_farm_position(caller)
                .update(|total_farm_position| *total_farm_position += &migrated_amount);
        }

        migrated_amount
    }

    fn decrease_old_farm_positions(&self, migrated_amount: BigUint, caller: &ManagedAddress) {
        if migrated_amount == BigUint::zero() {
            return;
        }

        let user_total_farm_position_mapper = self.user_total_farm_position(caller);
        let mut user_total_farm_position = user_total_farm_position_mapper.get();

        if user_total_farm_position > migrated_amount {
            user_total_farm_position -= &migrated_amount;
            user_total_farm_position_mapper.set(user_total_farm_position);
        } else {
            user_total_farm_position_mapper.clear();
        }
    }

    // Cannot import the one from farm, as the Wrapper struct has different dependencies
    fn claim_only_boosted_payment(&self, caller: &ManagedAddress) -> BigUint {
        let reward = FarmStakingWrapper::<Self>::calculate_boosted_rewards(self, caller);
        if reward > 0 {
            self.reward_reserve().update(|reserve| *reserve -= &reward);
        }

        reward
    }
}
