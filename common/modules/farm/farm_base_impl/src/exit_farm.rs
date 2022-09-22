elrond_wasm::imports!();

use crate::elrond_codec::TopEncode;
use common_structs::mergeable_token_traits::{
    CompoundedRewardAmountGetter, CurrentFarmAmountGetter, InitialFarmingAmountGetter,
};
use contexts::{
    exit_farm_context::ExitFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

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
    + token_merge_helper::TokenMergeHelperModule
    + farm_token_merge::FarmTokenMergeModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::base_farm_validation::BaseFarmValidationModule
    + crate::partial_positions::PartialPositionsModule
{
    fn exit_farm_base<
        AttributesType,
        GenerateAggregattedRewardsFunction,
        CalculateRewardsFunction,
    >(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
        generate_rewards_fn: GenerateAggregattedRewardsFunction,
        calculate_rewards_fn: CalculateRewardsFunction,
    ) -> InternalExitFarmResult<Self, AttributesType>
    where
        AttributesType: Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode
            + CurrentFarmAmountGetter<Self::Api>
            + InitialFarmingAmountGetter<Self::Api>
            + CompoundedRewardAmountGetter<Self::Api>,
        GenerateAggregattedRewardsFunction: Fn(&Self, &mut StorageCache<Self>),
        CalculateRewardsFunction:
            Fn(&Self, &BigUint, &AttributesType, &StorageCache<Self>) -> BigUint,
    {
        let mut storage_cache = StorageCache::new(self);
        let exit_farm_context =
            ExitFarmContext::new(payment, &storage_cache.farm_token_id, self.blockchain());

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        generate_rewards_fn(self, &mut storage_cache);

        let farm_token_amount = &exit_farm_context.farm_token.payment.amount;
        let attributes = &exit_farm_context.farm_token.attributes;
        let mut reward = calculate_rewards_fn(self, farm_token_amount, attributes, &storage_cache);
        storage_cache.reward_reserve -= &reward;

        let prev_compounded_rewards =
            self.calculate_previously_compounded_rewards(farm_token_amount, attributes);
        reward += prev_compounded_rewards;

        let initial_farming_amount =
            self.calculate_initial_farming_amount(farm_token_amount, attributes);

        self.burn_farm_token_payment(&exit_farm_context.farm_token.payment);

        let farming_token_payment = EsdtTokenPayment::new(
            storage_cache.farming_token_id.clone(),
            0,
            initial_farming_amount,
        );
        let reward_payment =
            EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

        InternalExitFarmResult {
            context: exit_farm_context,
            farming_token_payment,
            reward_payment,
            storage_cache,
        }
    }
}
