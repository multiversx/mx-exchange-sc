#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod base_functions;
pub mod external_interaction;

use base_functions::{ClaimRewardsResultType, DoubleMultiPayment, Wrapper};
use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;

use farm_base_impl::base_traits_impl::FarmContract;
use fixed_supply_token::FixedSupplyToken;

pub type EnterFarmResultType<M> = DoubleMultiPayment<M>;
pub type ExitFarmWithPartialPosResultType<M> = DoubleMultiPayment<M>;

pub const MAX_PERCENT: u64 = 10_000;

#[multiversx_sc::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + permissions_hub_module::PermissionsHubModule
    + original_owner_helper::OriginalOwnerHelperModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_functions::BaseFunctionsModule
    + external_interaction::ExternalInteractionsModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + farm_boosted_yields::custom_reward_logic::CustomRewardLogicModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        owner: ManagedAddress,
        timestamp_oracle_address: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        self.set_timestamp_oracle_address(timestamp_oracle_address);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set(current_epoch);
    }

    #[upgrade]
    fn upgrade(&self, timestamp_oracle_address: ManagedAddress) {
        if self.first_week_start_epoch().is_empty() {
            let current_epoch = self.blockchain().get_block_epoch();
            self.first_week_start_epoch().set(current_epoch);
        }

        // Farm position migration code
        let farm_token_mapper = self.farm_token();
        self.try_set_farm_position_migration_nonce(farm_token_mapper);

        self.set_timestamp_oracle_address(timestamp_oracle_address);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let new_farm_token = self.enter_farm::<Wrapper<Self>>(orig_caller.clone());
        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);
        self.update_energy_and_progress(&orig_caller);

        self.update_start_of_epoch_timestamp();

        (new_farm_token, boosted_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let claim_rewards_result = self.claim_rewards::<Wrapper<Self>>(orig_caller);
        self.send_payment_non_zero(&caller, &claim_rewards_result.new_farm_token);
        self.send_payment_non_zero(&caller, &claim_rewards_result.rewards);

        self.update_start_of_epoch_timestamp();

        claim_rewards_result.into()
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let output_farm_token_payment = self.compound_rewards::<Wrapper<Self>>(orig_caller.clone());
        self.send_payment_non_zero(&caller, &output_farm_token_payment);
        self.update_energy_and_progress(&orig_caller);

        self.update_start_of_epoch_timestamp();

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payment = self.call_value().single_esdt();
        let migrated_amount = self.migrate_old_farm_positions(&orig_caller);
        let exit_farm_result = self.exit_farm::<Wrapper<Self>>(orig_caller.clone(), payment);

        self.decrease_old_farm_positions(migrated_amount, &orig_caller);
        self.send_payment_non_zero(&caller, &exit_farm_result.farming_tokens);
        self.send_payment_non_zero(&caller, &exit_farm_result.rewards);
        self.clear_user_energy_if_needed(&orig_caller);

        self.update_start_of_epoch_timestamp();

        (exit_farm_result.farming_tokens, exit_farm_result.rewards).into()
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> DoubleMultiPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        self.migrate_old_farm_positions(&orig_caller);

        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let merged_farm_token = self.merge_and_update_farm_tokens(orig_caller);

        self.send_payment_non_zero(&caller, &merged_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);

        self.update_start_of_epoch_timestamp();

        (merged_farm_token, boosted_rewards_payment).into()
    }

    fn merge_and_update_farm_tokens(&self, orig_caller: ManagedAddress) -> EsdtTokenPayment {
        let mut output_attributes = self.merge_and_return_attributes::<Wrapper<Self>>(&orig_caller);
        output_attributes.original_owner = orig_caller;

        let new_token_amount = output_attributes.get_total_supply();
        self.farm_token()
            .nft_create(new_token_amount, &output_attributes)
    }

    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(
        &self,
        opt_user: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
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
        Wrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        let boosted_rewards = self.claim_only_boosted_payment(user);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        self.set_farm_supply_for_current_week(&storage_cache.farm_token_supply);

        self.send_payment_non_zero(user, &boosted_rewards_payment);

        // Don't need to call update here too, the internal functions call it already

        boosted_rewards_payment
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.end_produce_rewards::<Wrapper<Self>>();

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards_endpoint(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        self.set_per_block_rewards::<Wrapper<Self>>(per_block_amount);

        self.update_start_of_epoch_timestamp();
    }

    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_has_admin_permissions();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        let mut storage_cache = StorageCache::new(self);
        Wrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        self.boosted_yields_rewards_percentage().set(percentage);

        self.update_start_of_epoch_timestamp();
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        user: ManagedAddress,
        farm_token_amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        Wrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        Wrapper::<Self>::calculate_rewards(
            self,
            &user,
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
    }
}
