multiversx_sc::imports!();

use crate::base_traits_impl::FarmContract;
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

#[multiversx_sc::module]
pub trait BaseClaimRewardsModule:
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
    fn claim_rewards_base<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, FC::AttributesType> {
        let mut claim_result = self.claim_rewards_base_no_farm_token_mint::<FC>(caller, payments);
        let virtual_farm_token_payment = &claim_result.new_farm_token.payment;
        let minted_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token_payment.token_identifier,
            &virtual_farm_token_payment.amount,
            &claim_result.new_farm_token.attributes,
        );
        claim_result.new_farm_token.payment.token_nonce = minted_farm_token_nonce;

        claim_result
    }

    fn claim_rewards_base_no_farm_token_mint<FC: FarmContract<FarmSc = Self>>(
        &self,
        caller: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) -> InternalClaimRewardsResult<Self, FC::AttributesType> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let claim_rewards_context = ClaimRewardsContext::<Self::Api, FC::AttributesType>::new(
            payments,
            &storage_cache.farm_token_id,
            self.blockchain(),
        );

        FC::generate_aggregated_rewards(self, &mut storage_cache);

        let farm_token_amount = &claim_rewards_context.first_farm_token.payment.amount;
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
        let new_token_attributes = self.merge_attributes_from_payments(
            base_attributes,
            &claim_rewards_context.additional_payments,
            &farm_token_mapper,
        );
        let new_farm_token = PaymentAttributesPair {
            payment: EsdtTokenPayment::new(
                storage_cache.farm_token_id.clone(),
                0,
                new_token_attributes.get_total_supply(),
            ),
            attributes: new_token_attributes,
        };

        let first_farm_token = &claim_rewards_context.first_farm_token.payment;
        farm_token_mapper.nft_burn(first_farm_token.token_nonce, &first_farm_token.amount);
        self.send()
            .esdt_local_burn_multi(&claim_rewards_context.additional_payments);

        InternalClaimRewardsResult {
            created_with_merge: !claim_rewards_context.additional_payments.is_empty(),
            context: claim_rewards_context,
            rewards: EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward),
            new_farm_token,
            storage_cache,
        }
    }
}
