#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod base_functions;
pub mod exit_penalty;

use base_functions::{ClaimRewardsResultType, DoubleMultiPayment, Wrapper};
use common_structs::{Epoch, FarmTokenAttributes};
use contexts::storage_cache::StorageCache;

use exit_penalty::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
};
use farm_base_impl::base_traits_impl::FarmContract;

pub type ExitFarmWithPartialPosResultType<M> = DoubleMultiPayment<M>;

#[multiversx_sc::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_functions::BaseFunctionsModule
    + exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
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
        pair_contract_address: ManagedAddress,
        owner: ManagedAddress,
        first_week_start_epoch: Epoch,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            first_week_start_epoch >= current_epoch,
            "Invalid start epoch"
        );
        self.first_week_start_epoch().set(first_week_start_epoch);

        self.penalty_percent().set(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set(DEFAULT_BURN_GAS_LIMIT);
        self.pair_contract_address().set(&pair_contract_address);
    }

    #[endpoint]
    fn upgrade(&self) {
        // Farm position migration code
        let farm_token_mapper = self.farm_token();
        self.try_set_farm_position_migration_nonce(farm_token_mapper);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);
        self.add_boosted_rewards(&orig_caller, &boosted_rewards);

        let new_farm_token = self.enter_farm::<Wrapper<Self>>(orig_caller.clone());
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.update_energy_and_progress(&orig_caller);

        new_farm_token
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimRewardsResultType<Self::Api> {
        self.require_first_epoch_passed();

        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let claim_rewards_result = self.claim_rewards::<Wrapper<Self>>(orig_caller.clone());
        self.add_boosted_rewards(&orig_caller, &claim_rewards_result.rewards.boosted);

        let reward_token_id = self.reward_token_id().get();
        let base_rewards_payment =
            EsdtTokenPayment::new(reward_token_id, 0, claim_rewards_result.rewards.base);
        self.send_payment_non_zero(&caller, &claim_rewards_result.new_farm_token);
        self.send_payment_non_zero(&caller, &base_rewards_payment);

        (claim_rewards_result.new_farm_token, base_rewards_payment).into()
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
        self.add_boosted_rewards(&orig_caller, &exit_farm_result.rewards.boosted);

        self.decrease_old_farm_positions(migrated_amount, &orig_caller);

        let reward_token_id = self.reward_token_id().get();
        let base_rewards_payment =
            EsdtTokenPayment::new(reward_token_id, 0, exit_farm_result.rewards.base);
        self.send_payment_non_zero(&caller, &exit_farm_result.farming_tokens);
        self.send_payment_non_zero(&caller, &base_rewards_payment);

        self.clear_user_energy_if_needed(&orig_caller);

        (exit_farm_result.farming_tokens, base_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        self.migrate_old_farm_positions(&orig_caller);

        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);
        self.add_boosted_rewards(&orig_caller, &boosted_rewards);

        let merged_farm_token = self.merge_farm_tokens::<Wrapper<Self>>();
        self.send_payment_non_zero(&caller, &merged_farm_token);

        merged_farm_token
    }

    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(&self, opt_user: OptionalValue<ManagedAddress>) -> EsdtTokenPayment {
        self.require_first_epoch_passed();

        let caller = self.blockchain().get_caller();
        let user = match &opt_user {
            OptionalValue::Some(user) => user,
            OptionalValue::None => &caller,
        };
        let user_total_farm_position = self.get_user_total_farm_position(user);
        if user != &caller {
            require!(
                user_total_farm_position.allow_external_claim_boosted_rewards,
                "Cannot claim rewards for this address"
            );
        }

        let accumulated_boosted_rewards = self.accumulated_rewards_per_user(user).take();
        let boosted_rewards = self.claim_only_boosted_payment(user);
        let boosted_rewards_payment = EsdtTokenPayment::new(
            self.reward_token_id().get(),
            0,
            accumulated_boosted_rewards + boosted_rewards,
        );

        self.send_payment_non_zero(user, &boosted_rewards_payment);

        boosted_rewards_payment
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.end_produce_rewards::<Wrapper<Self>>();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards_endpoint(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        self.set_per_block_rewards::<Wrapper<Self>>(per_block_amount);
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

        let rewards = Wrapper::<Self>::calculate_rewards(
            self,
            &user,
            &farm_token_amount,
            &attributes,
            &storage_cache,
        );

        rewards.base
    }
}
