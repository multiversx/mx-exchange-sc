#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use common_errors::ERROR_ZERO_AMOUNT;
use common_structs::{FarmTokenAttributes, Nonce};
use contexts::storage_cache::StorageCache;

use farm_base_impl::base_traits_impl::{DefaultFarmWrapper, FarmContract};
use fixed_supply_token::FixedSupplyToken;
use weekly_rewards_splitting::ClaimProgress;

use crate::exit_penalty;

type EnterFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::module]
pub trait BaseFunctionsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    fn enter_farm<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> EnterFarmResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_enter_farm_result = self.enter_farm_base::<FC>(caller.clone(), payments);
        self.emit_enter_farm_event(
            &caller,
            base_enter_farm_result.context.farming_token_payment,
            base_enter_farm_result.new_farm_token.clone(),
            base_enter_farm_result.created_with_merge,
            base_enter_farm_result.storage_cache,
        );

        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(caller.clone());
        self.update_user_energy_for_current_week(&caller, current_week, &current_user_energy);
        self.current_claim_progress(&caller).set(ClaimProgress {
            energy: current_user_energy,
            week: current_week,
        });

        (
            base_enter_farm_result.new_farm_token.payment,
            base_enter_farm_result.boosted_rewards,
        )
            .into()
    }

    fn claim_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> ClaimRewardsResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_claim_rewards_result = self.claim_rewards_base::<FC>(caller.clone(), payments);

        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment.clone();
        let rewards_payment = base_claim_rewards_result.rewards;

        self.emit_claim_rewards_event(
            &caller,
            base_claim_rewards_result.context,
            base_claim_rewards_result.new_farm_token,
            rewards_payment.clone(),
            base_claim_rewards_result.created_with_merge,
            base_claim_rewards_result.storage_cache,
        );

        (output_farm_token_payment, rewards_payment).into()
    }

    fn compound_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_compound_rewards_result =
            self.compound_rewards_base::<FC>(caller.clone(), payments);

        let output_farm_token_payment = base_compound_rewards_result.new_farm_token.payment.clone();
        self.emit_compound_rewards_event(
            &caller,
            base_compound_rewards_result.context,
            base_compound_rewards_result.new_farm_token,
            base_compound_rewards_result.compounded_rewards,
            base_compound_rewards_result.created_with_merge,
            base_compound_rewards_result.storage_cache,
        );

        output_farm_token_payment
    }

    fn exit_farm<
        FC: FarmContract<FarmSc = Self, AttributesType = FarmTokenAttributes<Self::Api>>,
    >(
        &self,
        caller: ManagedAddress,
        exit_amount: BigUint,
    ) -> ExitFarmResultType<Self::Api> {
        let mut payment = self.call_value().single_esdt();

        require!(
            payment.amount >= exit_amount,
            "Exit amount is bigger than the payment amount"
        );
        let remaining_farm_payment = EsdtTokenPayment::new(
            payment.token_identifier.clone(),
            payment.token_nonce,
            &payment.amount - &exit_amount,
        );

        payment.amount = exit_amount;

        let base_exit_farm_result = self.exit_farm_base::<FC>(caller.clone(), payment);

        let mut farming_token_payment = base_exit_farm_result.farming_token_payment;
        let reward_payment = base_exit_farm_result.reward_payment;

        let initial_farm_token = base_exit_farm_result.context.farm_token.clone();
        if self.should_apply_penalty(initial_farm_token.attributes.entering_epoch) {
            self.burn_penalty(
                &mut farming_token_payment.amount,
                &base_exit_farm_result.storage_cache.farming_token_id,
                &base_exit_farm_result.storage_cache.reward_token_id,
            );
        }

        if remaining_farm_payment.amount == 0 {
            self.current_claim_progress(&caller).clear();
        }

        self.emit_exit_farm_event(
            &caller,
            base_exit_farm_result.context,
            farming_token_payment.clone(),
            reward_payment.clone(),
            base_exit_farm_result.storage_cache,
        );

        (
            farming_token_payment,
            reward_payment,
            remaining_farm_payment,
        )
            .into()
    }

    fn merge_farm_tokens(&self, caller: &ManagedAddress) -> EsdtTokenPayment<Self::Api> {
        self.check_claim_progress_for_merge(caller);
        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        let output_attributes: FarmTokenAttributes<Self::Api> =
            self.merge_from_payments_and_burn(payments, &token_mapper);
        let new_token_amount = output_attributes.get_total_supply().clone();
        token_mapper.nft_create(new_token_amount, &output_attributes)
    }

    fn check_claim_progress_for_merge(&self, caller: &ManagedAddress) {
        let claim_progress_mapper = self.current_claim_progress(caller);
        if !claim_progress_mapper.is_empty() {
            let current_week = self.get_current_week();
            let claim_progress = claim_progress_mapper.get();
            require!(
                claim_progress.week == current_week,
                "The user claim progress must be up to date."
            )
        }
    }

    fn end_produce_rewards<FC: FarmContract<FarmSc = Self>>(&self) {
        let mut storage = StorageCache::new(self);
        FC::generate_aggregated_rewards(self, &mut storage);

        self.produce_rewards_enabled().set(false);
    }

    fn set_per_block_rewards<FC: FarmContract<FarmSc = Self>>(&self, per_block_amount: BigUint) {
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        let mut storage = StorageCache::new(self);
        FC::generate_aggregated_rewards(self, &mut storage);

        self.per_block_reward_amount().set(&per_block_amount);
    }

    fn require_queried(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();
        require!(
            caller == sc_address,
            "May only call this function through VM query"
        );
    }
}

pub struct Wrapper<T: BaseFunctionsModule> {
    _phantom: PhantomData<T>,
}

impl<T> FarmContract for Wrapper<T>
where
    T: BaseFunctionsModule,
{
    type FarmSc = T;
    type AttributesType = FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>;

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    ) {
        let total_reward = Self::mint_per_block_rewards(sc, &storage_cache.reward_token_id);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;
            let split_rewards = sc.take_reward_slice(total_reward);

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }

    fn calculate_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_nonce: Nonce,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let base_farm_reward = DefaultFarmWrapper::<T>::calculate_rewards(
            sc,
            caller,
            farm_token_nonce,
            farm_token_amount,
            token_attributes,
            storage_cache,
        );
        let boosted_yield_rewards = Self::calculate_boosted_rewards(
            sc,
            caller,
            farm_token_nonce,
            farm_token_amount,
            storage_cache,
        );

        base_farm_reward + boosted_yield_rewards
    }

    fn calculate_boosted_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_nonce: Nonce,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let total_rewards_per_block = sc.per_block_reward_amount().get();
        sc.claim_boosted_yields_rewards(
            caller,
            farm_token_nonce,
            farm_token_amount,
            &storage_cache.farm_token_supply,
            &total_rewards_per_block,
        )
    }
}
