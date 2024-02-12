use common_errors::ERROR_DIFFERENT_TOKEN_IDS;
use common_structs::PaymentsVec;
use contexts::{claim_rewards_context::CompoundRewardsContext, storage_cache::StorageCache};
use farm_base_impl::compound_rewards::InternalCompoundRewardsResult;
use fixed_supply_token::FixedSupplyToken;

use crate::{farm_hooks::hook_type::FarmHookType, token_attributes::StakingFarmNftTokenAttributes};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CompoundStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
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
    + banned_addresses::BannedAddressModule
    + crate::farm_hooks::change_hooks::ChangeHooksModule
    + crate::farm_hooks::call_hook::CallHookModule
{
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeCompoundRewards,
            caller.clone(),
            payments,
            ManagedVec::new(),
        );

        let mut compound_result = self.compound_rewards_base(caller.clone(), payments_after_hook);

        let new_farm_token = compound_result.new_farm_token.payment.clone();
        let mut args = ManagedVec::new();
        self.encode_arg_to_vec(&compound_result.compounded_rewards, &mut args);

        let output_payments = self.call_hook(
            FarmHookType::AfterCompoundRewards,
            caller.clone(),
            ManagedVec::from_single_item(new_farm_token),
            args,
        );
        let new_farm_token = output_payments.get(0);
        self.send_payment_non_zero(&caller, &new_farm_token);

        compound_result.new_farm_token.payment = new_farm_token.clone();

        self.set_farm_supply_for_current_week(&compound_result.storage_cache.farm_token_supply);

        self.emit_compound_rewards_event(
            &caller,
            compound_result.context,
            compound_result.new_farm_token,
            compound_result.compounded_rewards,
            compound_result.created_with_merge,
            compound_result.storage_cache,
        );

        new_farm_token
    }

    fn compound_rewards_base(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self, StakingFarmNftTokenAttributes<Self::Api>> {
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
        let token_attributes = compound_rewards_context
            .first_farm_token
            .attributes
            .clone()
            .into_part(farm_token_amount);

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
        let new_farm_token = self.merge_and_create_token(
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
