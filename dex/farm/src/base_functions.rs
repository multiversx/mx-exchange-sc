#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use common_errors::ERROR_ZERO_AMOUNT;
use common_structs::{FarmToken, FarmTokenAttributes};
use contexts::storage_cache::StorageCache;

use farm_base_impl::{
    base_traits_impl::{FarmContract},
    exit_farm::InternalExitFarmResult,
};

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
    // farm boosted yields
    + farm_boosted_yields::FarmBoostedYieldsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    // + farm_base_impl::base_traits_impl::FarmContract
{
    fn enter_farm(&self, caller: ManagedAddress) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_enter_farm_result = self.enter_farm_base::<Wrapper::<Self>>(
            caller,
            payments,
        );

        base_enter_farm_result.new_farm_token.payment
    }

    fn claim_rewards(&self, caller: ManagedAddress) -> ClaimRewardsResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_claim_rewards_result = self.claim_rewards_base::<Wrapper::<Self>>(
            caller,
            payments,
        );

        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment;
        let rewards_payment = base_claim_rewards_result.rewards;
        (output_farm_token_payment, rewards_payment).into()
    }

    fn compound_rewards(&self, caller: ManagedAddress) -> EsdtTokenPayment<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let base_compound_rewards_result = self.compound_rewards_base::<Wrapper::<Self>>(
            caller,
            payments,
        );
        base_compound_rewards_result.new_farm_token.payment
    }

    fn exit_farm(&self, caller: ManagedAddress) -> ExitFarmResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
        let base_exit_farm_result: InternalExitFarmResult<Self, FarmTokenAttributes<Self::Api>> =
            self.exit_farm_base::<Wrapper::<Self>>(
                caller,
                payment,
            );

        let mut farming_token_payment = base_exit_farm_result.farming_token_payment;
        let reward_payment = base_exit_farm_result.reward_payment;

        let initial_farm_token = base_exit_farm_result.context.farm_token;
        if self.should_apply_penalty(initial_farm_token.attributes.entering_epoch) {
            self.burn_penalty(
                &mut farming_token_payment.amount,
                &base_exit_farm_result.storage_cache.farming_token_id,
                &base_exit_farm_result.storage_cache.reward_token_id,
            );
        }

        (farming_token_payment, reward_payment).into()
    }

    fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        // let payments = self.call_value().all_esdt_transfers();

        // let attrs = self.get_default_merged_farm_token_attributes(&payments, Option::None);
        // let farm_token_id = self.farm_token().get_token_id();
        // self.burn_farm_tokens_from_payments(&payments);

        // self.mint_farm_tokens(farm_token_id, attrs.current_farm_amount.clone(), &attrs)

        todo!();
    }

    fn end_produce_rewards(&self) {
        let mut storage = StorageCache::new(self);
        Wrapper::<Self>::generate_aggregated_rewards(self, &mut storage);

        //self.generate_aggregated_rewards_with_boosted_yields(&mut storage);
        self.produce_rewards_enabled().set(false);
    }

    fn set_per_block_rewards(&self, per_block_amount: BigUint) {
        require!(per_block_amount != 0u64, ERROR_ZERO_AMOUNT);

        let mut storage = StorageCache::new(self);
        Wrapper::<Self>::generate_aggregated_rewards(self, &mut storage);
        
        // self.generate_aggregated_rewards(&mut storage);
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

impl<T: BaseFunctionsModule> Wrapper<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData::<T>,
        }
    }
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
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let token_rps = token_attributes.get_reward_per_share();
        let base_farm_reward = if &storage_cache.reward_per_share > token_rps {
            let rps_diff = &storage_cache.reward_per_share - token_rps;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        };

        let boosted_yield_rewards =
            sc.claim_boosted_yields_rewards(&caller, &storage_cache.reward_token_id);

        base_farm_reward + boosted_yield_rewards
    }
}

// impl<A> FarmContract for ContractObj<A>
// where
//     A: elrond_wasm::api::VMApi,
//     ContractObj<A>: BaseFunctionsModule
//         + farm_boosted_yields::FarmBoostedYieldsModule
//         + week_timekeeping::WeekTimekeepingModule
//         + weekly_rewards_splitting::WeeklyRewardsSplittingModule
//         + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
//         + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
//         + energy_query::EnergyQueryModule
//         + utils::UtilsModule,
// {
//     type AttributesType = FarmTokenAttributes<Self::Api>;

//     fn generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
//         let total_reward =
//             self.mint_per_block_rewards(&storage_cache.reward_token_id, Self::mint_rewards);

//         if total_reward > 0u64 {
//             storage_cache.reward_reserve += &total_reward;
//             let split_rewards = self.take_reward_slice(total_reward);

//             if storage_cache.farm_token_supply != 0u64 {
//                 let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
//                     / &storage_cache.farm_token_supply;
//                 storage_cache.reward_per_share += &increase;
//             }
//         }
//     }

//     fn calculate_rewards(
//         &self,
//         farm_token_amount: &BigUint<Self::Api>,
//         token_attributes: &Self::AttributesType,
//         storage_cache: &StorageCache<Self>,
//     ) -> BigUint<Self::Api> {
//         let token_rps = token_attributes.get_reward_per_share();
//         let base_farm_reward = if &storage_cache.reward_per_share > token_rps {
//             let rps_diff = &storage_cache.reward_per_share - token_rps;
//             farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
//         } else {
//             BigUint::zero()
//         };

//         // TODO: Handle calls through proxy
//         let caller = self.blockchain().get_caller();
//         let boosted_yield_rewards =
//             self.claim_boosted_yields_rewards(&caller, &storage_cache.reward_token_id);

//         base_farm_reward + boosted_yield_rewards
//     }
// }
