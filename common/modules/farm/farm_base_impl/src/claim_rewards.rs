elrond_wasm::imports!();

use crate::elrond_codec::TopEncode;
use common_structs::{
    mergeable_token_traits::RewardPerShareGetter, DefaultFarmPaymentAttributesPair,
    FarmTokenAttributes, PaymentAttributesPair, PaymentsVec,
};
use contexts::{
    claim_rewards_context::ClaimRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

pub struct InternalClaimRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: ClaimRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub rewards: EsdtTokenPayment<C::Api>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub created_with_merge: bool,
}

#[elrond_wasm::module]
pub trait BaseClaimRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + token_merge_helper::TokenMergeHelperModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + admin_whitelist::AdminWhitelistModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::base_farm_validation::BaseFarmValidationModule
    + crate::partial_positions::PartialPositionsModule
{
    fn claim_rewards_base<
        AttributesType,
        GenerateAggregattedRewardsFunction,
        VirtualPositionCreatorFunction,
        AttributesMergingFunction,
        TokenMergingFunction,
    >(
        &self,
        payments: PaymentsVec<Self::Api>,
        generate_rewards_fn: GenerateAggregattedRewardsFunction,
        virtual_pos_create_fn: VirtualPositionCreatorFunction,
        attributes_merge_fn: AttributesMergingFunction,
        token_merge_fn: TokenMergingFunction,
    ) -> InternalClaimRewardsResult<Self, AttributesType>
    where
        AttributesType: Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode
            + RewardPerShareGetter<Self::Api>,
        GenerateAggregattedRewardsFunction: Fn(&mut StorageCache<Self>),
        VirtualPositionCreatorFunction: Fn(
            &PaymentAttributesPair<Self::Api, AttributesType>,
            &StorageCache<Self>,
        )
            -> PaymentAttributesPair<Self::Api, AttributesType>,
        AttributesMergingFunction: Fn(
            &ManagedVec<EsdtTokenPayment<Self::Api>>,
            Option<PaymentAttributesPair<Self::Api, AttributesType>>,
        ) -> AttributesType,
        TokenMergingFunction: Fn(
            PaymentAttributesPair<Self::Api, AttributesType>,
            &ManagedVec<EsdtTokenPayment<Self::Api>>,
            AttributesMergingFunction,
        ) -> PaymentAttributesPair<Self::Api, AttributesType>,
    {
        let mut storage_cache = StorageCache::new(self);
        let claim_rewards_context = ClaimRewardsContext::<Self::Api, AttributesType>::new(
            payments,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        generate_rewards_fn(&mut storage_cache);

        let farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
        let attributes = &claim_rewards_context.first_farm_token.attributes;
        let reward = self.default_calculate_reward(farm_token_amount, attributes, &storage_cache);
        storage_cache.reward_reserve -= &reward;

        let virtual_position =
            virtual_pos_create_fn(&claim_rewards_context.first_farm_token, &storage_cache);
        let new_farm_token = token_merge_fn(
            virtual_position,
            &claim_rewards_context.additional_payments,
            attributes_merge_fn,
        );

        self.burn_farm_token_payment(&claim_rewards_context.first_farm_token.payment);

        InternalClaimRewardsResult {
            created_with_merge: !claim_rewards_context.additional_payments.is_empty(),
            context: claim_rewards_context,
            rewards: EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward),
            new_farm_token,
            storage_cache,
        }
    }

    fn default_calculate_reward<AttributesType: RewardPerShareGetter<Self::Api>>(
        &self,
        farm_token_amount: &BigUint,
        farm_token_attributes: &AttributesType,
        storage_cache: &StorageCache<Self>,
    ) -> BigUint {
        let farm_token_reward_per_share = farm_token_attributes.get_reward_per_share();
        if &storage_cache.reward_per_share > farm_token_reward_per_share {
            let rps_diff = &storage_cache.reward_per_share - farm_token_reward_per_share;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        }
    }

    fn default_create_claim_rewards_virtual_position(
        &self,
        first_token: &DefaultFarmPaymentAttributesPair<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) -> DefaultFarmPaymentAttributesPair<Self::Api> {
        let farm_token_amount = first_token.payment.amount.clone();
        let initial_farming_amount =
            self.calculate_initial_farming_amount(&farm_token_amount, &first_token.attributes);
        let new_compound_reward_amount =
            self.calculate_new_compound_reward_amount(&farm_token_amount, &first_token.attributes);

        let virtual_position_token_amount = EsdtTokenPayment::new(
            storage_cache.farm_token_id.clone(),
            0,
            farm_token_amount.clone(),
        );
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: storage_cache.reward_per_share.clone(),
            entering_epoch: first_token.attributes.entering_epoch,
            original_entering_epoch: first_token.attributes.original_entering_epoch,
            initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: farm_token_amount,
        };

        PaymentAttributesPair {
            payment: virtual_position_token_amount,
            attributes: virtual_position_attributes,
        }
    }
}
