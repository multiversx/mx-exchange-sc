elrond_wasm::imports!();

use crate::{base_traits_impl::FarmContract, elrond_codec::TopEncode};
use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    enter_farm_context::EnterFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

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
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::base_farm_validation::BaseFarmValidationModule
    + utils::UtilsModule
{
    fn enter_farm_base(
        &self,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, Self::AttributesType>
    where
        Self: FarmContract,
    {
        let mut storage_cache = StorageCache::new(self);
        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_mapper = self.farm_token();
        let mut output_attributes = self.create_enter_farm_initial_attributes(
            enter_farm_context.farming_token_payment.amount.clone(),
            storage_cache.reward_per_share.clone(),
        );
        for payment in &enter_farm_context.additional_farm_tokens {
            let attributes: Self::AttributesType =
                self.get_attributes_as_part_of_fixed_supply(&payment, &farm_token_mapper);
            output_attributes.merge_with(attributes);
        }

        let new_farm_token_amount = output_attributes.get_total_supply().clone();
        let new_farm_token_payment =
            farm_token_mapper.nft_create(new_farm_token_amount, &output_attributes);
        let new_farm_token = PaymentAttributesPair {
            payment: new_farm_token_payment,
            attributes: output_attributes,
        };

        self.burn_multi_esdt(&enter_farm_context.additional_farm_tokens);

        InternalEnterFarmResult {
            created_with_merge: !enter_farm_context.additional_farm_tokens.is_empty(),
            context: enter_farm_context,
            storage_cache,
            new_farm_token,
        }
    }
}
