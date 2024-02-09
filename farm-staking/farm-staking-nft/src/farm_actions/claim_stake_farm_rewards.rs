multiversx_sc::imports!();

use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    claim_rewards_context::ClaimRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

use crate::{
    common::result_types::ClaimRewardsResultType,
    common::token_attributes::{
        PartialStakingFarmNftTokenAttributes, StakingFarmNftTokenAttributes,
    },
    farm_hooks::hook_type::FarmHookType,
};

pub struct InternalClaimRewardsResult<'a, C>
where
    C: FarmContracTraitBounds,
{
    pub context: ClaimRewardsContext<C::Api, StakingFarmNftTokenAttributes<C::Api>>,
    pub storage_cache: StorageCache<'a, C>,
    pub rewards: EsdtTokenPayment<C::Api>,
    pub new_farm_token: PaymentAttributesPair<C::Api, PartialStakingFarmNftTokenAttributes<C::Api>>,
    pub created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait ClaimStakeFarmRewardsModule:
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
    + crate::common::token_info::TokenInfoModule
{
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeClaimRewards,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            ManagedVec::new(),
        );
        let payment = payments_after_hook.get(0);

        let mut claim_result =
            self.claim_rewards_base(caller.clone(), ManagedVec::from_single_item(payment));
        let reward_nonce = self.reward_nonce().get();
        claim_result.rewards.token_nonce = reward_nonce;

        let mut virtual_farm_token = claim_result.new_farm_token.clone();

        self.update_energy_and_progress(&caller);

        let mut output_payments = ManagedVec::new();
        output_payments.push(virtual_farm_token.payment);
        self.push_if_non_zero_payment(&mut output_payments, claim_result.rewards.clone());

        let mut output_payments_after_hook = self.call_hook(
            FarmHookType::AfterClaimRewards,
            caller.clone(),
            output_payments,
            ManagedVec::new(),
        );
        virtual_farm_token.payment = self.pop_first_payment(&mut output_payments_after_hook);
        claim_result.rewards =
            self.pop_or_return_payment(&mut output_payments_after_hook, claim_result.rewards);

        self.send_payment_non_zero(&caller, &virtual_farm_token.payment);
        self.send_payment_non_zero(&caller, &claim_result.rewards);

        // self.emit_claim_rewards_event(
        //     &caller,
        //     claim_result.context,
        //     virtual_farm_token.clone(),
        //     claim_result.rewards.clone(),
        //     claim_result.created_with_merge,
        //     claim_result.storage_cache,
        // );

        ClaimRewardsResultType {
            new_farm_token: virtual_farm_token.payment,
            rewards: claim_result.rewards,
        }
    }

    fn claim_rewards_base(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self> {
        let mut claim_result = self.claim_rewards_base_no_farm_token_mint(caller, payments);
        let virtual_farm_token_payment = &claim_result.new_farm_token.payment;
        let minted_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token_payment.token_identifier,
            &virtual_farm_token_payment.amount,
            &claim_result.new_farm_token.attributes,
        );
        claim_result.new_farm_token.payment.token_nonce = minted_farm_token_nonce;

        claim_result
    }

    fn claim_rewards_base_no_farm_token_mint(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let claim_rewards_context =
            ClaimRewardsContext::<Self::Api, StakingFarmNftTokenAttributes<Self::Api>>::new(
                payments.clone(),
                &storage_cache.farm_token_id,
                self.blockchain(),
            );

        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
        let token_attributes = self.into_part(
            claim_rewards_context.first_farm_token.attributes.clone(),
            &claim_rewards_context.first_farm_token.payment,
        );

        let reward = self.calculate_rewards(
            &caller,
            farm_token_amount,
            &token_attributes,
            &storage_cache,
        );
        storage_cache.reward_reserve -= &reward;

        self.check_and_update_user_farm_position(&caller, &payments);

        let farm_token_mapper = self.farm_token();
        let base_attributes = self.create_claim_rewards_initial_attributes(
            caller,
            token_attributes,
            storage_cache.reward_per_share.clone(),
        );
        let new_token_attributes = self.merge_attributes_from_payments_nft(
            base_attributes,
            &claim_rewards_context.additional_payments,
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

        let first_farm_token = &claim_rewards_context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&claim_rewards_context.additional_payments);

        InternalClaimRewardsResult {
            created_with_merge: !claim_rewards_context.additional_payments.is_empty(),
            context: claim_rewards_context,
            rewards: EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward),
            new_farm_token,
            storage_cache,
        }
    }
}
