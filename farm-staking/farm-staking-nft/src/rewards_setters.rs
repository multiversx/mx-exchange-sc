use common_structs::Epoch;
use contexts::storage_cache::StorageCache;

multiversx_sc::imports!();

pub const MAX_MIN_UNBOND_EPOCHS: u64 = 30;

#[multiversx_sc::module]
pub trait RewardsSettersModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + crate::custom_rewards::CustomRewardsModule
{
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let payment = self.call_value().single_esdt();
        let reward_token_id = self.reward_token_id().get();
        let reward_nonce = self.reward_nonce().get();
        require!(
            payment.token_identifier == reward_token_id && payment.token_nonce == reward_nonce,
            "Invalid token"
        );

        self.reward_capacity().update(|r| *r += payment.amount);
    }

    #[payable("*")]
    #[endpoint(withdrawRewards)]
    fn withdraw_rewards(&self, withdraw_amount: BigUint) {
        self.require_caller_has_admin_permissions();

        self.reward_capacity().update(|rewards| {
            require!(
                *rewards >= withdraw_amount,
                "Not enough rewards to withdraw"
            );

            *rewards -= withdraw_amount.clone()
        });

        let caller = self.blockchain().get_caller();
        let reward_token_id = self.reward_token_id().get();
        let reward_nonce = self.reward_nonce().get();
        self.send_tokens_non_zero(&caller, &reward_token_id, reward_nonce, &withdraw_amount);
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards(&self) {
        self.require_caller_has_admin_permissions();

        let mut storage_cache = StorageCache::new(self);
        self.generate_aggregated_rewards(&mut storage_cache);
        self.produce_rewards_enabled().set(false);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(per_block_amount != 0, "Amount cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        self.generate_aggregated_rewards(&mut storage_cache);
        self.per_block_reward_amount().set(&per_block_amount);
    }

    #[endpoint(setMaxApr)]
    fn set_max_apr(&self, max_apr: BigUint) {
        self.require_caller_has_admin_permissions();
        require!(max_apr != 0, "Max APR cannot be zero");

        let mut storage_cache = StorageCache::new(self);
        self.generate_aggregated_rewards(&mut storage_cache);
        self.max_annual_percentage_rewards().set(&max_apr);
    }

    #[endpoint(setMinUnbondEpochs)]
    fn set_min_unbond_epochs_endpoint(&self, min_unbond_epochs: Epoch) {
        self.require_caller_has_admin_permissions();
        self.try_set_min_unbond_epochs(min_unbond_epochs);
    }

    fn try_set_min_unbond_epochs(&self, min_unbond_epochs: Epoch) {
        require!(
            min_unbond_epochs <= MAX_MIN_UNBOND_EPOCHS,
            "Invalid min unbond epochs"
        );

        self.min_unbond_epochs().set(min_unbond_epochs);
    }
}
