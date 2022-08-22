elrond_wasm::imports!();

use common_structs::{FarmToken, FarmTokenAttributes, PaymentsVec};
use contexts::{
    enter_farm_context::EnterFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

pub struct InternalEnterFarmResult<'a, C: FarmContracTraitBounds> {
    pub context: EnterFarmContext<C::Api>,
    pub storage_cache: StorageCache<'a, C>,
    pub new_farm_token: FarmToken<C::Api>,
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
        FarmTokenInstanceType,
        GenerateAggregattedRewardsFunction,
        VirtualPositionCreatorFunction,
        TokenMergingFunction,
    >(
        &self,
        payments: PaymentsVec<Self::Api>,
        generate_rewards_fn: GenerateAggregattedRewardsFunction,
        virtual_pos_create_fn: VirtualPositionCreatorFunction,
        token_merge_fn: TokenMergingFunction,
    ) -> InternalEnterFarmResult<Self>
    where
        GenerateAggregattedRewardsFunction: Fn(&mut StorageCache<Self>),
        VirtualPositionCreatorFunction:
            Fn(&EsdtTokenPayment<Self::Api>, &StorageCache<Self>) -> FarmTokenInstanceType,
        TokenMergingFunction: Fn(
            FarmTokenInstanceType,
            &ManagedVec<EsdtTokenPayment<Self::Api>>,
        ) -> FarmTokenInstanceType,
    {
        let mut storage_cache = StorageCache::new(self);
        let enter_farm_context = EnterFarmContext::new(
            payments,
            &storage_cache.farming_token_id,
            &storage_cache.farm_token_id,
        );

        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);
        generate_rewards_fn(&mut storage_cache);

        let block_epoch = self.blockchain().get_block_epoch();
        let first_payment_amount = enter_farm_context.farming_token_payment.amount.clone();
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: storage_cache.reward_per_share.clone(),
            entering_epoch: block_epoch,
            original_entering_epoch: block_epoch,
            initial_farming_amount: first_payment_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: first_payment_amount.clone(),
        };

        let virtual_position_token_amount =
            EsdtTokenPayment::new(storage_cache.farm_token_id.clone(), 0, first_payment_amount);
        let virtual_position = FarmToken {
            payment: virtual_position_token_amount,
            attributes: virtual_position_attributes,
        };
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            virtual_position,
            &enter_farm_context.additional_farm_tokens,
        );

        // let caller = self.blockchain().get_caller();
        let output_farm_token_payment = new_farm_token.payment.clone();
        // self.send_payment_non_zero(&caller, &output_farm_token_payment);

        // self.emit_enter_farm_event(
        //     enter_farm_context.farming_token_payment,
        //     new_farm_token,
        //     created_with_merge,
        //     storage_cache,
        // );

        output_farm_token_payment
    }

    fn create_enter_farm_temporary_token(
        &self,
        first_payment: &EsdtTokenPayment<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) -> FarmToken<Self::Api> {
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

        FarmToken {
            payment,
            attributes,
        }
    }
}
