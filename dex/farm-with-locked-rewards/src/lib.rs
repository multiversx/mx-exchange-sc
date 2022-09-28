#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{FarmTokenAttributes};
use contexts::storage_cache::StorageCache;

use farm::exit_penalty::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
};

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EgldOrEsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EgldOrEsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + locking_module::LockingModule
    + farm_token::FarmTokenModule
    + token_merge_helper::TokenMergeHelperModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm::base_functions::BaseFunctionsModule
    + farm::exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::partial_positions::PartialPositionsModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    // farm boosted yields
    + farm_boosted_yields::FarmBoostedYieldsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + energy_query::EnergyQueryModule
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
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(&self) -> EnterFarmResultType<Self::Api> {
        let output_farm_token_payment = self.enter_farm();
        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &output_farm_token_payment);
        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(&self) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);
        let (output_farm_token_payment, rewards_payment) = self.claim_rewards(&caller).into_tuple();
        self.send_payment_non_zero(&caller, &output_farm_token_payment);
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(caller, rewards_payment.token_identifier.clone(), rewards_payment.amount.clone());
        (output_farm_token_payment, locked_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards_endpoint(&self) -> CompoundRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);
        let output_farm_token_payment = self.compound_rewards(&caller);
        self.send_payment_non_zero(&caller, &output_farm_token_payment);
        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(&self) -> ExitFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);
        let (farming_token_payment, reward_payment) = self.exit_farm(&caller).into_tuple();
        self.send_payment_non_zero(&caller, &farming_token_payment);
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(caller, reward_payment.token_identifier.clone(), reward_payment.amount.clone());
        (farming_token_payment, locked_rewards_payment).into()
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
        self.generate_aggregated_rewards_with_boosted_yields(&mut storage_cache);

        self.calculate_reward_with_boosted_yields(&user, &farm_token_amount, &attributes, &storage_cache)
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);
        let new_tokens = self.merge_farm_tokens();
        self.send_payment_non_zero(&caller, &new_tokens);
        new_tokens
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.end_produce_rewards();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards_endpoint(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        self.set_per_block_rewards(per_block_amount);
    }

    fn send_to_lock_contract_non_zero(
        &self, 
        destination_address: ManagedAddress, 
        token_identifier: TokenIdentifier, 
        amount: BigUint
    ) -> EgldOrEsdtTokenPayment<Self::Api>
    {
        if amount == 0 {
            return EgldOrEsdtTokenPayment::no_payment();
        }
        let token_id = EgldOrEsdtTokenIdentifier::esdt(token_identifier);
        self.lock_tokens_and_forward(destination_address, token_id, amount)
    }
}
