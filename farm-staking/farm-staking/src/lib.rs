#![no_std]
#![feature(exact_size_is_empty)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::from_over_into)]
#![feature(trait_alias)]

use base_impl_wrapper::FarmStakingWrapper;
use common_structs::Nonce;
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;
use fixed_supply_token::FixedSupplyToken;
use token_attributes::StakingFarmTokenAttributes;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod base_impl_wrapper;
pub mod claim_stake_farm_rewards;
pub mod compound_stake_farm_rewards;
pub mod custom_rewards;
pub mod stake_farm;
pub mod token_attributes;
pub mod unbond_farm;
pub mod unstake_farm;

#[elrond_wasm::contract]
pub trait FarmStaking:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + stake_farm::StakeFarmModule
    + claim_stake_farm_rewards::ClaimStakeFarmRewardsModule
    + compound_stake_farm_rewards::CompoundStakeFarmRewardsModule
    + unstake_farm::UnstakeFarmModule
    + unbond_farm::UnbondFarmModule
{
    #[init]
    fn init(
        &self,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        max_apr: BigUint,
        min_unbond_epochs: u64,
        upgrade_block: Nonce,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        // farming and reward token are the same
        self.base_farm_init(
            farming_token_id.clone(),
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        require!(max_apr > 0u64, "Invalid max APR percentage");
        self.max_annual_percentage_rewards().set(&max_apr);

        self.try_set_min_unbond_epochs(min_unbond_epochs);

        let per_block_reward = self.per_block_reward_amount().get();
        let current_block_nonce = self.blockchain().get_block_epoch();
        let block_nonce_diff = current_block_nonce - upgrade_block;
        let rewards_since_upgrade = per_block_reward * block_nonce_diff;

        let accumulated_rewards_before = self.accumulated_rewards().update(|acc| {
            let before = (*acc).clone();
            *acc += &rewards_since_upgrade;

            let capacity = self.reward_capacity().get();
            *acc = core::cmp::min((*acc).clone(), capacity);

            before
        });
        self.reward_reserve()
            .update(|r| *r += accumulated_rewards_before);
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> EsdtTokenPayment<Self::Api> {
        let payments = self.get_non_empty_payments();
        let token_mapper = self.farm_token();
        let output_attributes: StakingFarmTokenAttributes<Self::Api> =
            self.merge_from_payments_and_burn(payments, &token_mapper);
        let new_token_amount = output_attributes.get_total_supply();
        let merged_farm_token = token_mapper.nft_create(new_token_amount, &output_attributes);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &merged_farm_token);

        merged_farm_token
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        farm_token_amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        FarmStakingWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        FarmStakingWrapper::<Self>::calculate_rewards(
            self,
            &ManagedAddress::zero(),
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
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
