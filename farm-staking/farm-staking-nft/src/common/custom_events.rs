multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{PaymentAttributesPair, PaymentsVec};
use contexts::{
    claim_rewards_context::CompoundRewardsContext,
    exit_farm_context::ExitFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

use super::token_attributes::StakingFarmNftTokenAttributes;

#[derive(TypeAbi, TopEncode)]
pub struct EnterFarmEvent<M: ManagedTypeApi> {
    farming_tokens: PaymentsVec<M>,
    farm_token: EsdtTokenPayment<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_reserve: BigUint<M>,
    farm_attributes: StakingFarmNftTokenAttributes<M>,
    created_with_merge: bool,
}

#[derive(TypeAbi, TopEncode)]
pub struct ExitFarmEvent<M: ManagedTypeApi> {
    farming_token_id: TokenIdentifier<M>,
    farming_token_amount: BigUint<M>,
    farm_token: EsdtTokenPayment<M>,
    farm_supply: BigUint<M>,
    reward_tokens: EsdtTokenPayment<M>,
    reward_reserve: BigUint<M>,
    farm_attributes: StakingFarmNftTokenAttributes<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct ClaimRewardsEvent<M: ManagedTypeApi> {
    old_farm_token: EsdtTokenPayment<M>,
    new_farm_token: EsdtTokenPayment<M>,
    farm_supply: BigUint<M>,
    reward_tokens: EsdtTokenPayment<M>,
    reward_reserve: BigUint<M>,
    old_farm_attributes: StakingFarmNftTokenAttributes<M>,
    new_farm_attributes: StakingFarmNftTokenAttributes<M>,
    created_with_merge: bool,
}

#[derive(TypeAbi, TopEncode)]
pub struct CompoundRewardsEvent<M: ManagedTypeApi> {
    old_farm_token: EsdtTokenPayment<M>,
    new_farm_token: EsdtTokenPayment<M>,
    farm_supply: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    compounded_reward: BigUint<M>,
    reward_reserve: BigUint<M>,
    old_farm_attributes: StakingFarmNftTokenAttributes<M>,
    new_farm_attributes: StakingFarmNftTokenAttributes<M>,
    created_with_merge: bool,
}

#[multiversx_sc::module]
pub trait CustomEventsModule {
    fn emit_enter_farm_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        caller: &ManagedAddress,
        input_farming_tokens: PaymentsVec<Self::Api>,
        output_farm_token: PaymentAttributesPair<
            Self::Api,
            StakingFarmNftTokenAttributes<Self::Api>,
        >,
        created_with_merge: bool,
        storage_cache: StorageCache<'a, C>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();

        self.enter_farm_event(
            caller,
            epoch,
            block,
            timestamp,
            &storage_cache.farming_token_id,
            &EnterFarmEvent {
                farming_tokens: input_farming_tokens,
                farm_token: output_farm_token.payment,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: storage_cache.reward_token_id.clone(),
                reward_token_reserve: storage_cache.reward_reserve.clone(),
                farm_attributes: output_farm_token.attributes,
                created_with_merge,
            },
        )
    }

    fn emit_exit_farm_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        caller: &ManagedAddress,
        exit_farm_context: ExitFarmContext<Self::Api, StakingFarmNftTokenAttributes<Self::Api>>,
        output_farming_tokens: EsdtTokenPayment<Self::Api>,
        output_reward: EsdtTokenPayment<Self::Api>,
        storage_cache: StorageCache<'a, C>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();

        self.exit_farm_event(
            caller,
            epoch,
            block,
            timestamp,
            &storage_cache.farm_token_id,
            &ExitFarmEvent {
                farming_token_id: output_farming_tokens.token_identifier,
                farming_token_amount: output_farming_tokens.amount,
                farm_token: exit_farm_context.farm_token.payment,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_tokens: output_reward,
                reward_reserve: storage_cache.reward_reserve.clone(),
                farm_attributes: exit_farm_context.farm_token.attributes,
            },
        )
    }

    fn emit_claim_rewards_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        &self,
        caller: &ManagedAddress,
        input_farm_token: PaymentAttributesPair<
            Self::Api,
            StakingFarmNftTokenAttributes<Self::Api>,
        >,
        output_farm_token: PaymentAttributesPair<
            Self::Api,
            StakingFarmNftTokenAttributes<Self::Api>,
        >,
        output_reward: EsdtTokenPayment<Self::Api>,
        created_with_merge: bool,
        storage_cache: StorageCache<'a, C>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();

        self.claim_rewards_event(
            caller,
            epoch,
            block,
            timestamp,
            &storage_cache.farm_token_id,
            &ClaimRewardsEvent {
                old_farm_token: input_farm_token.payment,
                new_farm_token: output_farm_token.payment,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_tokens: output_reward,
                reward_reserve: storage_cache.reward_reserve.clone(),
                old_farm_attributes: input_farm_token.attributes,
                new_farm_attributes: output_farm_token.attributes,
                created_with_merge,
            },
        )
    }

    fn emit_compound_rewards_event<'a, C: FarmContracTraitBounds<Api = Self::Api>>(
        self,
        caller: &ManagedAddress,
        compound_rewards_context: CompoundRewardsContext<
            Self::Api,
            StakingFarmNftTokenAttributes<Self::Api>,
        >,
        output_farm_token: PaymentAttributesPair<
            Self::Api,
            StakingFarmNftTokenAttributes<Self::Api>,
        >,
        compounded_reward_amount: BigUint,
        created_with_merge: bool,
        storage_cache: StorageCache<'a, C>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();

        self.compound_rewards_event(
            caller,
            epoch,
            block,
            timestamp,
            &storage_cache.farm_token_id,
            &CompoundRewardsEvent {
                old_farm_token: compound_rewards_context.first_farm_token.payment,
                new_farm_token: output_farm_token.payment,
                farm_supply: storage_cache.farm_token_supply.clone(),
                reward_token_id: storage_cache.reward_token_id.clone(),
                compounded_reward: compounded_reward_amount,
                reward_reserve: storage_cache.reward_reserve.clone(),
                old_farm_attributes: compound_rewards_context.first_farm_token.attributes,
                new_farm_attributes: output_farm_token.attributes,
                created_with_merge,
            },
        )
    }

    #[event("enter_farm")]
    fn enter_farm_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        #[indexed] farming_token: &TokenIdentifier,
        enter_farm_event: &EnterFarmEvent<Self::Api>,
    );

    #[event("exit_farm")]
    fn exit_farm_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        #[indexed] farm_token: &TokenIdentifier,
        exit_farm_event: &ExitFarmEvent<Self::Api>,
    );

    #[event("claim_rewards")]
    fn claim_rewards_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        #[indexed] farm_token: &TokenIdentifier,
        claim_rewards_event: &ClaimRewardsEvent<Self::Api>,
    );

    #[event("compound_rewards")]
    fn compound_rewards_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        #[indexed] farm_token: &TokenIdentifier,
        compound_rewards_event: &CompoundRewardsEvent<Self::Api>,
    );
}
