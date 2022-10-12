elrond_wasm::imports!();

use crate::{base_traits_impl::FarmContract, elrond_codec::TopEncode};
use common_structs::FarmToken;
use contexts::{
    exit_farm_context::ExitFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};
use fixed_supply_token::FixedSupplyToken;

pub struct InternalExitFarmResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: ExitFarmContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub farming_token_payment: EsdtTokenPayment<C::Api>,
    pub reward_payment: EsdtTokenPayment<C::Api>,
}

#[elrond_wasm::module]
pub trait BaseExitFarmModule:
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
    fn exit_farm_base(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> InternalExitFarmResult<Self, Self::AttributesType>
    where
        Self: FarmContract,
    {
        let mut storage_cache = StorageCache::new(self);
        let exit_farm_context = ExitFarmContext::<Self::Api, Self::AttributesType>::new(
            payment,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_amount = &exit_farm_context.farm_token.payment.amount;
        let token_attributes = exit_farm_context
            .farm_token
            .attributes
            .clone()
            .into_part(farm_token_amount);

        let mut reward =
            self.calculate_rewards(farm_token_amount, &token_attributes, &storage_cache);
        storage_cache.reward_reserve -= &reward;
        reward += token_attributes.get_compounded_rewards();

        let farming_token_payment = EsdtTokenPayment::new(
            storage_cache.farming_token_id.clone(),
            0,
            token_attributes.get_initial_farming_tokens().clone(),
        );
        let reward_payment =
            EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

        let farm_token_payment = &exit_farm_context.farm_token.payment;
        self.send().esdt_local_burn(
            &farm_token_payment.token_identifier,
            farm_token_payment.token_nonce,
            &farm_token_payment.amount,
        );

        InternalExitFarmResult {
            context: exit_farm_context,
            farming_token_payment,
            reward_payment,
            storage_cache,
        }
    }
}
