multiversx_sc::imports!();

use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{enter_farm_context::EnterFarmContext, storage_cache::StorageCache};
use farm_base_impl::enter_farm::InternalEnterFarmResult;

use crate::{
    common::result_types::EnterFarmResultType,
    common::token_attributes::PartialStakingFarmNftTokenAttributes,
};

#[multiversx_sc::module]
pub trait StakeFarmModule:
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
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(&self) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();

        let boosted_rewards = self.claim_only_boosted_payment(&caller);
        let reward_nonce = self.reward_nonce().get();
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), reward_nonce, boosted_rewards);

        let farm_token_mapper = self.farm_token();
        let farming_token_id = self.farming_token_id().get();
        let farm_token_id = farm_token_mapper.get_token_id();
        let mut total_farming_token = BigUint::zero();
        let mut all_farming_tokens = PaymentsVec::new();
        let mut other_farm_tokens = PaymentsVec::new();
        for payment in &payments {
            if payment.token_identifier == farm_token_id {
                other_farm_tokens.push(payment);
            } else if payment.token_identifier == farming_token_id {
                total_farming_token += &payment.amount;
                all_farming_tokens.push(payment);
            } else {
                sc_panic!("Invalid payments");
            }
        }

        require!(total_farming_token > 0, "No farming tokens");

        let farming_token_payment = EsdtTokenPayment::new(farming_token_id, 0, total_farming_token);
        let mut enter_input_payments = PaymentsVec::from_single_item(farming_token_payment);
        enter_input_payments.append_vec(other_farm_tokens);

        let enter_result =
            self.enter_farm_base_no_token_create(caller.clone(), enter_input_payments);

        let new_farm_token = enter_result.new_farm_token.payment.clone();
        let mut attributes = enter_result.new_farm_token.attributes;
        attributes
            .farming_token_parts
            .append_vec(all_farming_tokens.clone());
        let attr_full = attributes.clone().into_full();

        let new_farm_token = farm_token_mapper.nft_create(new_farm_token.amount, &attr_full);
        self.total_supply(new_farm_token.token_nonce)
            .set(&new_farm_token.amount);
        self.remaining_supply(new_farm_token.token_nonce)
            .set(&new_farm_token.amount);
        self.remaining_parts(new_farm_token.token_nonce)
            .set(&attr_full.farming_token_parts);

        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&caller, &boosted_rewards_payment);

        self.set_farm_supply_for_current_week(&enter_result.storage_cache.farm_token_supply);
        self.update_energy_and_progress(&caller);

        let output_token = PaymentAttributesPair {
            payment: new_farm_token.clone(),
            attributes: attr_full,
        };
        self.emit_enter_farm_event(
            &caller,
            all_farming_tokens,
            output_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        EnterFarmResultType {
            new_farm_token,
            boosted_rewards_payment,
        }
    }

    fn enter_farm_base_no_token_create(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, PartialStakingFarmNftTokenAttributes<Self::Api>> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        // The order is important - first check and update, then increase position
        self.check_and_update_user_farm_position(
            &caller,
            &enter_farm_context.additional_farm_tokens,
        );
        self.increase_user_farm_position(&caller, &enter_farm_context.farming_token_payment.amount);

        self.generate_aggregated_rewards(&mut storage_cache);

        storage_cache.farm_token_supply += &enter_farm_context.farming_token_payment.amount;

        let farm_token_mapper = self.farm_token();
        let base_attributes = self.create_enter_farm_initial_attributes(
            caller,
            enter_farm_context.farming_token_payment.amount.clone(),
            storage_cache.reward_per_share.clone(),
        );
        let new_token_attributes = self.merge_attributes_from_payments_nft(
            base_attributes,
            &enter_farm_context.additional_farm_tokens,
            &farm_token_mapper,
        );
        let new_farm_token = PaymentAttributesPair {
            payment: EsdtTokenPayment::new(
                storage_cache.farm_token_id.clone(),
                0,
                new_token_attributes.current_farm_amount.clone(),
            ),
            attributes: new_token_attributes,
        };

        self.send()
            .esdt_local_burn_multi(&enter_farm_context.additional_farm_tokens);

        InternalEnterFarmResult {
            created_with_merge: !enter_farm_context.additional_farm_tokens.is_empty(),
            context: enter_farm_context,
            storage_cache,
            new_farm_token,
        }
    }
}
