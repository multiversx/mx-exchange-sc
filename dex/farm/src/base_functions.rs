#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use common_errors::ERROR_ZERO_AMOUNT;
use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;

use farm_base_impl::base_traits_impl::{DefaultFarmWrapper, FarmContract};
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

use crate::exit_penalty;

type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

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
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    fn enter_farm<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_enter_farm_result = self.enter_farm_base::<FC>(caller.clone(), payments);

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
    ) -> ExitFarmResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
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

        self.emit_exit_farm_event(
            &caller,
            base_exit_farm_result.context,
            farming_token_payment.clone(),
            reward_payment.clone(),
            base_exit_farm_result.storage_cache,
        );

        (farming_token_payment, reward_payment).into()
    }

    fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        let mut payments = self.get_non_empty_payments();
        let first_payment = self.pop_first_payment(&mut payments);

        let token_mapper = self.farm_token();
        let mut output_attributes: FarmTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&first_payment, &token_mapper);
        token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        for payment in &payments {
            let attributes: FarmTokenAttributes<Self::Api> =
                self.get_attributes_as_part_of_fixed_supply(&payment, &token_mapper);
            output_attributes.merge_with(attributes);
        }

        self.burn_multi_esdt(&payments);

        let new_token_amount = output_attributes.get_total_supply().clone();
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
        let total_reward =
            sc.mint_per_block_rewards(&storage_cache.reward_token_id, Self::mint_rewards);

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
        let boosted_yield_rewards =
            sc.claim_boosted_yields_rewards(caller, &storage_cache.reward_token_id);

        base_farm_reward + boosted_yield_rewards
    }
}
