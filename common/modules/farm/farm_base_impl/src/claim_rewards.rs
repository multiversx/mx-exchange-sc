elrond_wasm::imports!();

use crate::{base_traits_impl::FarmContract, elrond_codec::TopEncode};
use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    claim_rewards_context::ClaimRewardsContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};
use fixed_supply_token::FixedSupplyToken;

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
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::base_farm_validation::BaseFarmValidationModule
    + utils::UtilsModule
{
    fn claim_rewards_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, FC::AttributesType> {
        let mut storage_cache = StorageCache::new(self);
        let claim_rewards_context = ClaimRewardsContext::<Self::Api, FC::AttributesType>::new(
            payments,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
        let farm_token_nonce = claim_rewards_context.first_farm_token.payment.token_nonce;
        let token_attributes = claim_rewards_context
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

        let farm_token_mapper = self.farm_token();
        let base_attributes = FC::create_claim_rewards_initial_attributes(
            self,
            caller,
            token_attributes,
            storage_cache.reward_per_share.clone(),
        );
        let new_farm_token = self.merge_and_create_token(
            base_attributes,
            &claim_rewards_context.additional_payments,
            &farm_token_mapper,
        );

        let first_farm_token = &claim_rewards_context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.burn_multi_esdt(&claim_rewards_context.additional_payments);

        InternalClaimRewardsResult {
            created_with_merge: !claim_rewards_context.additional_payments.is_empty(),
            context: claim_rewards_context,
            rewards: EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward),
            new_farm_token,
            storage_cache,
        }
    }
}
