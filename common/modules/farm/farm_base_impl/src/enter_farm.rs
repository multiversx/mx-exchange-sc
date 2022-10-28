elrond_wasm::imports!();

use crate::{base_traits_impl::FarmContract, elrond_codec::TopEncode};
use common_structs::{PaymentAttributesPair, PaymentsVec};
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
    pub boosted_rewards: EsdtTokenPayment<C::Api>,
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
    fn enter_farm_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalEnterFarmResult<Self, FC::AttributesType> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let reward = if !enter_farm_context.additional_farm_tokens.is_empty() {
            let payment = enter_farm_context.additional_farm_tokens.get(0);
            FC::calculate_boosted_rewards(
                self,
                &caller,
                payment.token_nonce,
                &payment.amount,
                &storage_cache,
            )
        } else {
            BigUint::zero()
        };

        storage_cache.reward_reserve -= &reward;
        let boosted_rewards =
            EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

        storage_cache.farm_token_supply += &enter_farm_context.farming_token_payment.amount;

        let farm_token_mapper = self.farm_token();
        let base_attributes = FC::create_enter_farm_initial_attributes(
            self,
            caller,
            enter_farm_context.farming_token_payment.amount.clone(),
            storage_cache.reward_per_share.clone(),
        );
        let new_farm_token = self.merge_and_create_token(
            base_attributes,
            &enter_farm_context.additional_farm_tokens,
            &farm_token_mapper,
        );

        self.burn_multi_esdt(&enter_farm_context.additional_farm_tokens);

        InternalEnterFarmResult {
            created_with_merge: !enter_farm_context.additional_farm_tokens.is_empty(),
            context: enter_farm_context,
            storage_cache,
            new_farm_token,
            boosted_rewards,
        }
    }
}
