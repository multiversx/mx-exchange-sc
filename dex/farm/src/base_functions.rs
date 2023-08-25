#![allow(clippy::too_many_arguments)]
#![allow(clippy::from_over_into)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use core::marker::PhantomData;

use common_errors::ERROR_ZERO_AMOUNT;
use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;

use farm_base_impl::base_traits_impl::{DefaultFarmWrapper, FarmContract};
use fixed_supply_token::FixedSupplyToken;

use crate::exit_penalty;

pub type DoubleMultiPayment<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type ClaimRewardsResultType<M> = DoubleMultiPayment<M>;
pub type ExitFarmResultType<M> = DoubleMultiPayment<M>;

pub struct ClaimRewardsResultWrapper<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
    pub rewards: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub farming_tokens: EsdtTokenPayment<M>,
    pub rewards: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> Into<ClaimRewardsResultType<M>> for ClaimRewardsResultWrapper<M> {
    fn into(self) -> ClaimRewardsResultType<M> {
        (self.new_farm_token, self.rewards).into()
    }
}

impl<M: ManagedTypeApi> Into<ExitFarmResultType<M>> for ExitFarmResultWrapper<M> {
    fn into(self) -> ExitFarmResultType<M> {
        (self.farming_tokens, self.rewards).into()
    }
}

#[multiversx_sc::module]
pub trait BaseFunctionsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
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
    fn enter_farm<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> EsdtTokenPayment {
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let base_enter_farm_result = self.enter_farm_base::<FC>(caller.clone(), payments);

        self.set_farm_supply_for_current_week(
            &base_enter_farm_result.storage_cache.farm_token_supply,
        );

        self.emit_enter_farm_event(
            &caller,
            base_enter_farm_result.context.farming_token_payment,
            base_enter_farm_result.new_farm_token.clone(),
            base_enter_farm_result.created_with_merge,
            base_enter_farm_result.storage_cache,
        );

        base_enter_farm_result.new_farm_token.payment
    }

    fn claim_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> ClaimRewardsResultWrapper<Self::Api> {
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let base_claim_rewards_result = self.claim_rewards_base::<FC>(caller.clone(), payments);

        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment.clone();
        let rewards_payment = base_claim_rewards_result.rewards;

        self.set_farm_supply_for_current_week(
            &base_claim_rewards_result.storage_cache.farm_token_supply,
        );

        self.emit_claim_rewards_event(
            &caller,
            base_claim_rewards_result.context,
            base_claim_rewards_result.new_farm_token,
            rewards_payment.clone(),
            base_claim_rewards_result.created_with_merge,
            base_claim_rewards_result.storage_cache,
        );

        ClaimRewardsResultWrapper {
            new_farm_token: output_farm_token_payment,
            rewards: rewards_payment,
        }
    }

    fn compound_rewards<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers().clone_value();
        let base_compound_rewards_result =
            self.compound_rewards_base::<FC>(caller.clone(), payments);

        let output_farm_token_payment = base_compound_rewards_result.new_farm_token.payment.clone();

        self.set_farm_supply_for_current_week(
            &base_compound_rewards_result.storage_cache.farm_token_supply,
        );

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

    fn exit_farm<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payment: EsdtTokenPayment,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let base_exit_farm_result = self.exit_farm_base::<FC>(caller.clone(), payment);

        let mut farming_token_payment = base_exit_farm_result.farming_token_payment;
        let reward_payment = base_exit_farm_result.reward_payment;

        self.set_farm_supply_for_current_week(
            &base_exit_farm_result.storage_cache.farm_token_supply,
        );

        FC::apply_penalty(
            self,
            &mut farming_token_payment.amount,
            &base_exit_farm_result.context.farm_token.attributes,
            &base_exit_farm_result.storage_cache,
        );

        self.emit_exit_farm_event(
            &caller,
            base_exit_farm_result.context,
            farming_token_payment.clone(),
            reward_payment.clone(),
            base_exit_farm_result.storage_cache,
        );

        ExitFarmResultWrapper {
            farming_tokens: farming_token_payment,
            rewards: reward_payment,
        }
    }

    fn merge_farm_tokens<FC: FarmContract<FarmSc = Self>>(&self) -> EsdtTokenPayment<Self::Api> {
        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        token_mapper.require_all_same_token(&payments);

        let output_attributes: FC::AttributesType =
            self.merge_from_payments_and_burn(payments, &token_mapper);
        let new_token_amount = output_attributes.get_total_supply();
        token_mapper.nft_create(new_token_amount, &output_attributes)
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

pub struct Wrapper<
    T: BaseFunctionsModule
        + farm_boosted_yields::FarmBoostedYieldsModule
        + crate::exit_penalty::ExitPenaltyModule,
> {
    _phantom: PhantomData<T>,
}

impl<T> Wrapper<T>
where
    T: BaseFunctionsModule
        + farm_boosted_yields::FarmBoostedYieldsModule
        + crate::exit_penalty::ExitPenaltyModule,
{
    pub fn calculate_boosted_rewards(
        sc: &<Self as FarmContract>::FarmSc,
        caller: &ManagedAddress<<<Self as FarmContract>::FarmSc as ContractBase>::Api>,
        token_attributes: &<Self as FarmContract>::AttributesType,
        farm_token_amount: BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api>,
    ) -> BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api> {
        if &token_attributes.original_owner != caller {
            sc.update_energy_and_progress(caller);

            return BigUint::zero();
        }

        sc.claim_boosted_yields_rewards(caller, farm_token_amount)
    }
}

impl<T> FarmContract for Wrapper<T>
where
    T: BaseFunctionsModule
        + farm_boosted_yields::FarmBoostedYieldsModule
        + crate::exit_penalty::ExitPenaltyModule,
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
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let base_farm_reward = DefaultFarmWrapper::<T>::calculate_rewards(
            sc,
            caller,
            farm_token_amount,
            token_attributes,
            storage_cache,
        );
        let boosted_yield_rewards = Self::calculate_boosted_rewards(
            sc,
            caller,
            token_attributes,
            farm_token_amount.clone(),
        );

        base_farm_reward + boosted_yield_rewards
    }

    fn get_exit_penalty(
        sc: &Self::FarmSc,
        total_exit_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let current_epoch = sc.blockchain().get_block_epoch();
        let user_farming_epochs = current_epoch - token_attributes.entering_epoch;
        let min_farming_epochs = sc.minimum_farming_epochs().get();
        if user_farming_epochs >= min_farming_epochs {
            BigUint::zero()
        } else {
            total_exit_amount * sc.penalty_percent().get() / exit_penalty::MAX_PERCENT
        }
    }

    fn apply_penalty(
        sc: &Self::FarmSc,
        total_exit_amount: &mut BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) {
        let penalty_amount = Self::get_exit_penalty(sc, total_exit_amount, token_attributes);
        if penalty_amount > 0 {
            *total_exit_amount -= &penalty_amount;

            sc.burn_farming_tokens(
                &penalty_amount,
                &storage_cache.farming_token_id,
                &storage_cache.reward_token_id,
            );
        }
    }
}
