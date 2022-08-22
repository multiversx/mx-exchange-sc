elrond_wasm::imports!();

use crate::elrond_codec::TopEncode;
use common_structs::{
    mergeable_token_traits::CurrentFarmAmountGetter, FarmTokenAttributes, PaymentAttributesPair,
    PaymentsVec,
};
use contexts::{
    enter_farm_context::EnterFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

pub struct InternalEnterFarmResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: EnterFarmContext<C::Api>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub created_with_merge: bool,
}

#[elrond_wasm::module]
pub trait BaseEnterFarmModule:
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
{
    fn enter_farm_base<
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
    ) -> InternalEnterFarmResult<Self, AttributesType>
    where
        AttributesType: Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode
            + CurrentFarmAmountGetter<Self::Api>,
        GenerateAggregattedRewardsFunction: Fn(&mut StorageCache<Self>),
        VirtualPositionCreatorFunction: Fn(
            &EsdtTokenPayment<Self::Api>,
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
        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        generate_rewards_fn(&mut storage_cache);

        let virtual_position =
            virtual_pos_create_fn(&enter_farm_context.farming_token_payment, &storage_cache);
        let new_farm_token = token_merge_fn(
            virtual_position,
            &enter_farm_context.additional_farm_tokens,
            attributes_merge_fn,
        );

        // let caller = self.blockchain().get_caller();
        // let output_farm_token_payment = new_farm_token.payment.clone();
        // self.send_payment_non_zero(&caller, &output_farm_token_payment);

        // self.emit_enter_farm_event(
        //     enter_farm_context.farming_token_payment,
        //     new_farm_token,
        //     created_with_merge,
        //     storage_cache,
        // );

        InternalEnterFarmResult {
            created_with_merge: !enter_farm_context.additional_farm_tokens.is_empty(),
            context: enter_farm_context,
            storage_cache,
            new_farm_token,
        }
    }

    fn default_create_enter_farm_virtual_position(
        &self,
        first_payment: &EsdtTokenPayment<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) -> PaymentAttributesPair<Self::Api, FarmTokenAttributes<Self::Api>> {
        let block_epoch = self.blockchain().get_block_epoch();
        let attributes = FarmTokenAttributes {
            reward_per_share: storage_cache.reward_per_share.clone(),
            entering_epoch: block_epoch,
            original_entering_epoch: block_epoch,
            initial_farming_amount: first_payment.amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: first_payment.amount.clone(),
        };
        let payment = EsdtTokenPayment::new(
            storage_cache.farm_token_id.clone(),
            0,
            first_payment.amount.clone(),
        );

        PaymentAttributesPair {
            payment,
            attributes,
        }
    }
}
