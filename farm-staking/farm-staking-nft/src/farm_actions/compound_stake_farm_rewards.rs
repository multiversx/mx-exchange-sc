use common_errors::ERROR_DIFFERENT_TOKEN_IDS;
use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    claim_rewards_context::CompoundRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

use crate::{
    common::result_types::CompoundRewardsResultType,
    common::token_attributes::StakingFarmNftTokenAttributes,
};

multiversx_sc::imports!();

pub struct InternalCompoundRewardsResult<'a, C>
where
    C: FarmContracTraitBounds,
{
    pub context: CompoundRewardsContext<C::Api, StakingFarmNftTokenAttributes<C::Api>>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, StakingFarmNftTokenAttributes<C::Api>>,
    pub compounded_rewards: BigUint<C::Api>,
    pub created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait CompoundStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
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
    + crate::common::token_info::TokenInfoModule
    + crate::common::custom_events::CustomEventsModule
{
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> CompoundRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();

        let mut compound_result = self.compound_rewards_base(caller.clone(), payments);

        let new_farm_token = compound_result.new_farm_token.payment.clone();
        self.total_supply(new_farm_token.token_nonce)
            .set(&new_farm_token.amount);
        self.remaining_supply(new_farm_token.token_nonce)
            .set(&new_farm_token.amount);
        self.remaining_parts(new_farm_token.token_nonce).set(
            &compound_result
                .new_farm_token
                .attributes
                .farming_token_parts,
        );

        self.send_payment_non_zero(&caller, &new_farm_token);

        compound_result.new_farm_token.payment = new_farm_token.clone();

        self.set_farm_supply_for_current_week(&compound_result.storage_cache.farm_token_supply);

        self.emit_compound_rewards_event(
            &caller,
            compound_result.context,
            compound_result.new_farm_token,
            compound_result.compounded_rewards.clone(),
            compound_result.created_with_merge,
            compound_result.storage_cache,
        );

        CompoundRewardsResultType {
            new_farm_token,
            compounded_rewards: compound_result.compounded_rewards,
        }
    }

    fn compound_rewards_base(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        require!(
            storage_cache.farming_token_id == storage_cache.reward_token_id,
            ERROR_DIFFERENT_TOKEN_IDS
        );

        let compound_rewards_context =
            CompoundRewardsContext::<Self::Api, StakingFarmNftTokenAttributes<Self::Api>>::new(
                payments.clone(),
                &storage_cache.farm_token_id,
                self.blockchain(),
            );

        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_amount = &compound_rewards_context.first_farm_token.payment.amount;
        let token_attributes = self.into_part(
            compound_rewards_context.first_farm_token.attributes.clone(),
            &compound_rewards_context.first_farm_token.payment,
        );

        let reward = self.calculate_rewards(
            &caller,
            farm_token_amount,
            &token_attributes,
            &storage_cache,
        );
        storage_cache.reward_reserve -= &reward;
        storage_cache.farm_token_supply += &reward;

        self.check_and_update_user_farm_position(&caller, &payments);

        let farm_token_mapper = self.farm_token();
        let base_attributes = self.create_compound_rewards_initial_attributes(
            caller.clone(),
            token_attributes,
            storage_cache.reward_per_share.clone(),
            &reward,
        );
        let new_farm_token = self.merge_and_create_token_nft(
            base_attributes,
            &compound_rewards_context.additional_payments,
            &farm_token_mapper,
        );

        self.increase_user_farm_position(&caller, &reward);

        let first_farm_token = &compound_rewards_context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&compound_rewards_context.additional_payments);

        InternalCompoundRewardsResult {
            created_with_merge: !compound_rewards_context.additional_payments.is_empty(),
            context: compound_rewards_context,
            new_farm_token,
            compounded_rewards: reward,
            storage_cache,
        }
    }
}
