elrond_wasm::imports!();

use common_structs::mergeable_token_traits::{
    CompoundedRewardAmountGetter, CurrentFarmAmountGetter, InitialFarmingAmountGetter,
};

#[elrond_wasm::module]
pub trait PartialPositionsModule: token_merge_helper::TokenMergeHelperModule {
    fn calculate_initial_farming_amount<AttributesType>(
        &self,
        farm_token_amount: &BigUint,
        farm_token_attributes: &AttributesType,
    ) -> BigUint
    where
        AttributesType: CurrentFarmAmountGetter<Self::Api> + InitialFarmingAmountGetter<Self::Api>,
    {
        self.rule_of_three_non_zero_result(
            farm_token_amount,
            farm_token_attributes.get_current_farm_amount(),
            farm_token_attributes.get_initial_farming_amount(),
        )
    }

    fn calculate_new_compound_reward_amount<AttributesType>(
        &self,
        farm_token_amount: &BigUint,
        farm_token_attributes: &AttributesType,
    ) -> BigUint
    where
        AttributesType:
            CurrentFarmAmountGetter<Self::Api> + CompoundedRewardAmountGetter<Self::Api>,
    {
        self.rule_of_three(
            farm_token_amount,
            farm_token_attributes.get_current_farm_amount(),
            farm_token_attributes.get_compounded_reward_amount(),
        )
    }
}
