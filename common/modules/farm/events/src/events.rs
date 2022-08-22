#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{DefaultFarmPaymentAttributesPair, FarmTokenAttributes};
use contexts::{
    claim_rewards_context::{ClaimRewardsContext, CompoundRewardsContext},
    exit_farm_context::ExitFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

#[derive(TypeAbi, TopEncode)]
pub struct EnterFarmEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farming_token_id: TokenIdentifier<M>,
    farming_token_amount: BigUint<M>,
    farm_token_id: TokenIdentifier<M>,
    farm_token_nonce: u64,
    farm_token_amount: BigUint<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_reserve: BigUint<M>,
    farm_attributes: FarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct ExitFarmEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farming_token_id: TokenIdentifier<M>,
    farming_token_amount: BigUint<M>,
    farm_token_id: TokenIdentifier<M>,
    farm_token_nonce: u64,
    farm_token_amount: BigUint<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_nonce: u64,
    reward_token_amount: BigUint<M>,
    reward_reserve: BigUint<M>,
    farm_attributes: FarmTokenAttributes<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct ClaimRewardsEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    old_farm_token_id: TokenIdentifier<M>,
    old_farm_token_nonce: u64,
    old_farm_token_amount: BigUint<M>,
    new_farm_token_id: TokenIdentifier<M>,
    new_farm_token_nonce: u64,
    new_farm_token_amount: BigUint<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_nonce: u64,
    reward_token_amount: BigUint<M>,
    reward_reserve: BigUint<M>,
    old_farm_attributes: FarmTokenAttributes<M>,
    new_farm_attributes: FarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct CompoundRewardsEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    old_farm_token_id: TokenIdentifier<M>,
    old_farm_token_nonce: u64,
    old_farm_token_amount: BigUint<M>,
    new_farm_token_id: TokenIdentifier<M>,
    new_farm_token_nonce: u64,
    new_farm_token_amount: BigUint<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_nonce: u64,
    reward_token_amount: BigUint<M>,
    reward_reserve: BigUint<M>,
    old_farm_attributes: FarmTokenAttributes<M>,
    new_farm_attributes: FarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_enter_farm_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        input_farming_token: EsdtTokenPayment<Self::Api>,
        output_farm_token: DefaultFarmPaymentAttributesPair<Self::Api>,
        created_with_merge: bool,
        storage_cache: StorageCache<'a, C>,
    ) {
        let caller = self.blockchain().get_caller();
        let block_nonce = self.blockchain().get_block_nonce();
        let block_epoch = self.blockchain().get_block_epoch();
        let block_timestamp = self.blockchain().get_block_timestamp();

        self.enter_farm_event(
            &caller.clone(),
            &storage_cache.farming_token_id,
            block_epoch,
            &EnterFarmEvent {
                caller,
                farming_token_id: input_farming_token.token_identifier,
                farming_token_amount: input_farming_token.amount,
                farm_token_id: output_farm_token.payment.token_identifier,
                farm_token_nonce: output_farm_token.payment.token_nonce,
                farm_token_amount: output_farm_token.payment.amount,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: storage_cache.reward_token_id.clone(),
                reward_token_reserve: storage_cache.reward_reserve.clone(),
                farm_attributes: output_farm_token.attributes,
                created_with_merge,
                block: block_nonce,
                epoch: block_epoch,
                timestamp: block_timestamp,
            },
        )
    }

    fn emit_exit_farm_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        exit_farm_context: ExitFarmContext<Self::Api, FarmTokenAttributes<Self::Api>>,
        output_farming_tokens: EsdtTokenPayment<Self::Api>,
        output_reward: EsdtTokenPayment<Self::Api>,
        storage_cache: StorageCache<'a, C>,
    ) {
        let caller = self.blockchain().get_caller();
        let block_nonce = self.blockchain().get_block_nonce();
        let block_epoch = self.blockchain().get_block_epoch();
        let block_timestamp = self.blockchain().get_block_timestamp();

        self.exit_farm_event(
            &caller.clone(),
            &storage_cache.farm_token_id,
            block_epoch,
            &ExitFarmEvent {
                caller,
                farming_token_id: output_farming_tokens.token_identifier,
                farming_token_amount: output_farming_tokens.amount,
                farm_token_id: exit_farm_context.farm_token.payment.token_identifier,
                farm_token_nonce: exit_farm_context.farm_token.payment.token_nonce,
                farm_token_amount: exit_farm_context.farm_token.payment.amount,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: output_reward.token_identifier,
                reward_token_nonce: output_reward.token_nonce,
                reward_token_amount: output_reward.amount,
                reward_reserve: storage_cache.reward_reserve.clone(),
                farm_attributes: exit_farm_context.farm_token.attributes,
                block: block_nonce,
                epoch: block_epoch,
                timestamp: block_timestamp,
            },
        )
    }

    fn emit_claim_rewards_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        claim_rewards_context: ClaimRewardsContext<Self::Api, FarmTokenAttributes<Self::Api>>,
        output_farm_token: DefaultFarmPaymentAttributesPair<Self::Api>,
        created_with_merge: bool,
        output_reward: EsdtTokenPayment<Self::Api>,
        storage_cache: StorageCache<'a, C>,
    ) {
        let caller = self.blockchain().get_caller();
        let block_nonce = self.blockchain().get_block_nonce();
        let block_epoch = self.blockchain().get_block_epoch();
        let block_timestamp = self.blockchain().get_block_timestamp();

        self.claim_rewards_event(
            &caller.clone(),
            &storage_cache.farm_token_id,
            block_epoch,
            &ClaimRewardsEvent {
                caller,
                old_farm_token_id: claim_rewards_context
                    .first_farm_token
                    .payment
                    .token_identifier,
                old_farm_token_nonce: claim_rewards_context.first_farm_token.payment.token_nonce,
                old_farm_token_amount: claim_rewards_context.first_farm_token.payment.amount,
                new_farm_token_id: output_farm_token.payment.token_identifier,
                new_farm_token_nonce: output_farm_token.payment.token_nonce,
                new_farm_token_amount: output_farm_token.payment.amount,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: output_reward.token_identifier,
                reward_token_nonce: output_reward.token_nonce,
                reward_token_amount: output_reward.amount,
                reward_reserve: storage_cache.reward_reserve.clone(),
                old_farm_attributes: claim_rewards_context.first_farm_token.attributes,
                new_farm_attributes: output_farm_token.attributes,
                created_with_merge,
                block: block_nonce,
                epoch: block_epoch,
                timestamp: block_timestamp,
            },
        )
    }

    fn emit_compound_rewards_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        self,
        compound_rewards_context: CompoundRewardsContext<Self::Api, FarmTokenAttributes<Self::Api>>,
        output_farm_token: DefaultFarmPaymentAttributesPair<Self::Api>,
        created_with_merge: bool,
        compounded_reward_amount: BigUint,
        storage_cache: StorageCache<'a, C>,
    ) {
        let caller = self.blockchain().get_caller();
        let block_nonce = self.blockchain().get_block_nonce();
        let block_epoch = self.blockchain().get_block_epoch();
        let block_timestamp = self.blockchain().get_block_timestamp();

        self.compound_rewards_event(
            &caller.clone(),
            &storage_cache.farm_token_id,
            block_epoch,
            &CompoundRewardsEvent {
                caller,
                old_farm_token_id: compound_rewards_context
                    .first_farm_token
                    .payment
                    .token_identifier,
                old_farm_token_nonce: compound_rewards_context
                    .first_farm_token
                    .payment
                    .token_nonce,
                old_farm_token_amount: compound_rewards_context.first_farm_token.payment.amount,
                new_farm_token_id: output_farm_token.payment.token_identifier,
                new_farm_token_nonce: output_farm_token.payment.token_nonce,
                new_farm_token_amount: output_farm_token.payment.amount,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: storage_cache.reward_token_id.clone(),
                reward_token_nonce: 0,
                reward_token_amount: compounded_reward_amount,
                reward_reserve: storage_cache.reward_reserve.clone(),
                old_farm_attributes: compound_rewards_context.first_farm_token.attributes,
                new_farm_attributes: output_farm_token.attributes,
                created_with_merge,
                block: block_nonce,
                epoch: block_epoch,
                timestamp: block_timestamp,
            },
        )
    }

    #[event("enter_farm")]
    fn enter_farm_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farming_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        enter_farm_event: &EnterFarmEvent<Self::Api>,
    );

    #[event("exit_farm")]
    fn exit_farm_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        exit_farm_event: &ExitFarmEvent<Self::Api>,
    );

    #[event("claim_rewards")]
    fn claim_rewards_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        claim_rewards_event: &ClaimRewardsEvent<Self::Api>,
    );

    #[event("compound_rewards")]
    fn compound_rewards_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        compound_rewards_event: &CompoundRewardsEvent<Self::Api>,
    );
}
