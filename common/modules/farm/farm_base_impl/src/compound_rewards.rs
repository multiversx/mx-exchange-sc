multiversx_sc::imports!();

use crate::base_traits_impl::FarmContract;
use common_errors::ERROR_DIFFERENT_TOKEN_IDS;
use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    claim_rewards_context::CompoundRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};
use fixed_supply_token::FixedSupplyToken;

pub struct InternalCompoundRewardsResult<'a, C, T>
where
    C: FarmContracTraitBounds,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
{
    pub context: CompoundRewardsContext<C::Api, T>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: PaymentAttributesPair<C::Api, T>,
    pub compounded_rewards: BigUint<C::Api>,
    pub created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait BaseCompoundRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::base_farm_validation::BaseFarmValidationModule
    + utils::UtilsModule
{
    fn compound_rewards_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalCompoundRewardsResult<Self, FC::AttributesType> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        require!(
            storage_cache.farming_token_id == storage_cache.reward_token_id,
            ERROR_DIFFERENT_TOKEN_IDS
        );

        let compound_rewards_context = CompoundRewardsContext::<Self::Api, FC::AttributesType>::new(
            payments,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let farm_token_amount = &compound_rewards_context.first_farm_token.payment.amount;
        let token_attributes = compound_rewards_context
            .first_farm_token
            .attributes
            .clone()
            .into_part(farm_token_amount);

        let reward = FC::calculate_rewards(
            self,
            &caller,
            farm_token_amount,
            &token_attributes,
            &storage_cache,
        );
        storage_cache.reward_reserve -= &reward;
        storage_cache.farm_token_supply += &reward;

        let farm_token_mapper = self.farm_token();
        let base_attributes = FC::create_compound_rewards_initial_attributes(
            self,
            caller,
            token_attributes,
            storage_cache.reward_per_share.clone(),
            &reward,
        );
        let new_farm_token = self.merge_and_create_token(
            base_attributes,
            &compound_rewards_context.additional_payments,
            &farm_token_mapper,
        );

        let first_farm_token = &compound_rewards_context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&compound_rewards_context.additional_payments);

        InternalCompoundRewardsResult {
            created_with_merge: !compound_rewards_context.additional_payments.is_empty(),
            context: compound_rewards_context,
            new_farm_token,
            compounded_rewards: reward,
            storage_cache,
        }
    }
}
