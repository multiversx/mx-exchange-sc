#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod base_functions;
pub mod claim_boost_only;
pub mod exit_penalty;
pub mod progress_update;

use base_functions::{ClaimRewardsResultType, DoubleMultiPayment, Wrapper};
use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;

use exit_penalty::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
};
use farm_base_impl::base_traits_impl::FarmContract;
use mergeable::Mergeable;

pub type EnterFarmResultType<M> = DoubleMultiPayment<M>;
pub type ExitFarmWithPartialPosResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;

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
    + progress_update::ProgressUpdateModule
    + claim_boost_only::ClaimBoostOnlyModule
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
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        self.penalty_percent().set_if_empty(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.pair_contract_address().set(&pair_contract_address);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        let payments = self.get_non_empty_payments();
        let first_additional_payment_index = 1;
        let boosted_rewards = match payments.try_get(first_additional_payment_index) {
            Some(p) => self.claim_only_boosted_payment(&orig_caller, &p),
            None => EsdtTokenPayment::new(self.reward_token_id().get(), 0, BigUint::zero()),
        };

        let new_farm_token = self.enter_farm::<Wrapper<Self>>(orig_caller.clone());
        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards);

        self.update_energy_and_progress(&orig_caller);

        (new_farm_token, boosted_rewards).into()
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        let claim_rewards_result = self.claim_rewards::<Wrapper<Self>>(orig_caller);
        self.send_payment_non_zero(&caller, &claim_rewards_result.new_farm_token);
        self.send_payment_non_zero(&caller, &claim_rewards_result.rewards);

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

        let output_farm_token_payment = self.compound_rewards::<Wrapper<Self>>(orig_caller);
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(
        &self,
        exit_amount: BigUint,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        let mut payment = self.call_value().single_esdt();
        require!(
            payment.amount >= exit_amount,
            "Exit amount is bigger than the payment amount"
        );

        let boosted_rewards_full_position = self.claim_only_boosted_payment(&orig_caller, &payment);
        let remaining_farm_payment = EsdtTokenPayment::new(
            payment.token_identifier.clone(),
            payment.token_nonce,
            &payment.amount - &exit_amount,
        );

        payment.amount = exit_amount;

        let mut exit_farm_result = self.exit_farm::<Wrapper<Self>>(orig_caller.clone(), payment);
        exit_farm_result
            .rewards
            .merge_with(boosted_rewards_full_position);

        self.send_payment_non_zero(&caller, &exit_farm_result.farming_tokens);
        self.send_payment_non_zero(&caller, &exit_farm_result.rewards);
        self.send_payment_non_zero(&caller, &remaining_farm_payment);

        self.clear_user_energy_if_needed(&orig_caller, &remaining_farm_payment.amount);

        (
            exit_farm_result.farming_tokens,
            exit_farm_result.rewards,
            remaining_farm_payment,
        )
            .into()
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

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        self.check_claim_progress_for_merge(&orig_caller);

        let merged_farm_token = self.merge_farm_tokens::<Wrapper<Self>>();
        self.send_payment_non_zero(&caller, &merged_farm_token);

        merged_farm_token
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
}
