#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod base_farm_init;
pub mod base_farm_validation;
pub mod claim_rewards;
pub mod compound_rewards;
pub mod enter_farm;
pub mod exit_farm;
pub mod partial_positions;

use claim_rewards::InternalClaimRewardsResult;
use common_structs::{FarmTokenAttributes, PaymentsVec, Energy};
use compound_rewards::InternalCompoundRewardsResult;
use contexts::storage_cache::StorageCache;
use enter_farm::InternalEnterFarmResult;
use exit_farm::InternalExitFarmResult;

pub type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::module]
pub trait FarmBaseImpl:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + token_merge_helper::TokenMergeHelperModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_farm_init::BaseFarmInitModule
    + base_farm_validation::BaseFarmValidationModule
    + partial_positions::PartialPositionsModule
    + enter_farm::BaseEnterFarmModule
    + claim_rewards::BaseClaimRewardsModule
    + compound_rewards::BaseCompoundRewardsModule
    + exit_farm::BaseExitFarmModule
{
    fn default_enter_farm_impl(
        &self,
        original_user: &ManagedAddress,
        energy: Energy<Self::Api>,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, FarmTokenAttributes<Self::Api>> {
        self.enter_farm_base(
            original_user,
            energy,
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_create_enter_farm_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    fn default_claim_rewards_impl(
        &self,
        caller: &ManagedAddress,
        energy: Energy<Self::Api>,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, FarmTokenAttributes<Self::Api>> {
        self.claim_rewards_base(
            caller,
            energy,
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
            Self::default_create_claim_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    // TODO: Think about reusing some of the logic from claim_rewards
    // How to fix: Don't mint tokens, and allow caller to do what they wish with token/attributes
    fn default_compound_rewards_impl(
        &self,
        caller: &ManagedAddress,
        energy: Energy<Self::Api>,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self, FarmTokenAttributes<Self::Api>> {
        self.compound_rewards_base(
            caller,
            energy,
            payments,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
            Self::default_create_compound_rewards_virtual_position,
            Self::get_default_merged_farm_token_attributes,
            Self::create_farm_tokens_by_merging,
        )
    }

    fn default_exit_farm_impl(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> InternalExitFarmResult<Self, FarmTokenAttributes<Self::Api>> {
        self.exit_farm_base(
            payment,
            Self::default_generate_aggregated_rewards,
            Self::default_calculate_reward,
        )
    }

    fn default_generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
        let mint_function = |token_id: &TokenIdentifier, amount: &BigUint| {
            self.send().esdt_local_mint(token_id, 0, amount);
        };
        let total_reward =
            self.mint_per_block_rewards(&storage_cache.reward_token_id, mint_function);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }
}
