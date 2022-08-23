elrond_wasm::imports!();

use crate::elrond_codec::TopEncode;
use common_errors::ERROR_DIFFERENT_TOKEN_IDS;
use common_structs::{
    DefaultFarmPaymentAttributesPair, FarmTokenAttributes, PaymentAttributesPair, PaymentsVec,
};
use contexts::{
    claim_rewards_context::CompoundRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

pub struct InternalCompoundRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: CompoundRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub created_with_merge: bool,
}

#[elrond_wasm::module]
pub trait BaseCompoundRewardsModule:
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
    fn compound_rewards_base<
        AttributesType,
        GenerateAggregattedRewardsFunction,
        CalculateRewardsFunction,
        VirtualPositionCreatorFunction,
        AttributesMergingFunction,
        TokenMergingFunction,
    >(
        &self,
        payments: PaymentsVec<Self::Api>,
        generate_rewards_fn: GenerateAggregattedRewardsFunction,
        calculate_rewards_fn: CalculateRewardsFunction,
        virtual_pos_create_fn: VirtualPositionCreatorFunction,
        attributes_merge_fn: AttributesMergingFunction,
        token_merge_fn: TokenMergingFunction,
    ) -> InternalCompoundRewardsResult<Self, AttributesType>
    where
        AttributesType: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
        GenerateAggregattedRewardsFunction: Fn(&Self, &mut StorageCache<Self>),
        CalculateRewardsFunction:
            Fn(&Self, &BigUint, &AttributesType, &StorageCache<Self>) -> BigUint,
        VirtualPositionCreatorFunction: Fn(
            &Self,
            &PaymentAttributesPair<Self::Api, AttributesType>,
            &StorageCache<Self>,
            &BigUint,
        )
            -> PaymentAttributesPair<Self::Api, AttributesType>,
        AttributesMergingFunction: Fn(
            &Self,
            &PaymentsVec<Self::Api>,
            Option<PaymentAttributesPair<Self::Api, AttributesType>>,
        ) -> AttributesType,
        TokenMergingFunction: Fn(
            &Self,
            PaymentAttributesPair<Self::Api, AttributesType>,
            &PaymentsVec<Self::Api>,
            AttributesMergingFunction,
        ) -> PaymentAttributesPair<Self::Api, AttributesType>,
    {
        let mut storage_cache = StorageCache::new(self);
        let compound_rewards_context = CompoundRewardsContext::<Self::Api, AttributesType>::new(
            payments,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        require!(
            storage_cache.farming_token_id == storage_cache.reward_token_id,
            ERROR_DIFFERENT_TOKEN_IDS
        );

        generate_rewards_fn(self, &mut storage_cache);

        let farm_token_amount = &compound_rewards_context.first_farm_token.payment.amount;
        let attributes = &compound_rewards_context.first_farm_token.attributes;
        let reward = calculate_rewards_fn(self, farm_token_amount, attributes, &storage_cache);
        storage_cache.reward_reserve -= &reward;

        let virtual_position = virtual_pos_create_fn(
            self,
            &compound_rewards_context.first_farm_token,
            &storage_cache,
            &reward,
        );
        let new_farm_token = token_merge_fn(
            self,
            virtual_position,
            &compound_rewards_context.additional_payments,
            attributes_merge_fn,
        );

        self.burn_farm_token_payment(&compound_rewards_context.first_farm_token.payment);

        // self.emit_compound_rewards_event(
        //     compound_rewards_context,
        //     new_farm_token,
        //     created_with_merge,
        //     reward,
        //     storage_cache,
        // );

        InternalCompoundRewardsResult {
            created_with_merge: !compound_rewards_context.additional_payments.is_empty(),
            context: compound_rewards_context,
            new_farm_token,
            storage_cache,
        }
    }

    fn default_create_compound_rewards_virtual_position(
        &self,
        first_token: &DefaultFarmPaymentAttributesPair<Self::Api>,
        storage_cache: &StorageCache<Self>,
        reward: &BigUint,
    ) -> DefaultFarmPaymentAttributesPair<Self::Api> {
        let farm_token_amount = first_token.payment.amount.clone();
        let initial_farming_amount =
            self.calculate_initial_farming_amount(&farm_token_amount, &first_token.attributes);
        let new_compound_reward_amount =
            self.calculate_new_compound_reward_amount(&farm_token_amount, &first_token.attributes);

        let virtual_position_amount = &farm_token_amount + reward;
        let virtual_position_token_amount = EsdtTokenPayment::new(
            storage_cache.farm_token_id.clone(),
            0,
            virtual_position_amount,
        );

        let block_epoch = self.blockchain().get_block_epoch();
        let virtual_position_compounded_reward = &new_compound_reward_amount + reward;
        let virtual_position_current_farm_amount = &farm_token_amount + reward;
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: storage_cache.reward_per_share.clone(),
            entering_epoch: block_epoch,
            original_entering_epoch: block_epoch,
            initial_farming_amount,
            compounded_reward: virtual_position_compounded_reward,
            current_farm_amount: virtual_position_current_farm_amount,
        };

        PaymentAttributesPair {
            payment: virtual_position_token_amount,
            attributes: virtual_position_attributes,
        }
    }
}
