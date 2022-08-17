#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use contexts::storage_cache::{FarmContracTraitBounds, StorageCache};
use farm_token::FarmToken;

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
        output_farm_token: FarmToken<Self::Api>,
        created_with_merge: bool,
        storage_cache: StorageCache<'a, C>,
    ) {
        let caller = self.blockchain().get_caller();
        let block_nonce = self.blockchain().get_block_nonce();
        let block_epoch = self.blockchain().get_block_epoch();
        let block_timestamp = self.blockchain().get_block_timestamp();

        self.enter_farm_event(
            &caller.clone(),
            &input_farming_token.token_identifier.clone(),
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

    /*
    fn emit_exit_farm_event(&self, ctx: &GenericContext<Self::Api>) {
        let first_pay = &ctx.get_tx_input().first_payment;
        let reward = match ctx.get_final_reward() {
            Some(rew) => rew.clone(),
            None => {
                EsdtTokenPayment::new(TokenIdentifier::from_esdt_bytes(&[]), 0, BigUint::zero())
            }
        };

        self.exit_farm_event(
            ctx.get_caller(),
            ctx.get_farm_token_id(),
            ctx.get_block_epoch(),
            &ExitFarmEvent {
                caller: ctx.get_caller().clone(),
                farming_token_id: ctx.get_farming_token_id().clone(),
                farming_token_amount: ctx.get_initial_farming_amount().clone(),
                farm_token_id: ctx.get_farm_token_id().clone(),
                farm_token_nonce: first_pay.token_nonce,
                farm_token_amount: first_pay.amount.clone(),
                farm_supply: ctx.get_farm_token_supply().clone(),
                reward_token_id: reward.token_identifier,
                reward_token_nonce: reward.token_nonce,
                reward_token_amount: reward.amount,
                reward_reserve: ctx.get_reward_reserve().clone(),
                farm_attributes: ctx.get_input_attributes().clone(),
                block: ctx.get_block_nonce(),
                epoch: ctx.get_block_epoch(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_claim_rewards_event(&self, ctx: &GenericContext<Self::Api>) {
        let first_pay = &ctx.get_tx_input().first_payment;
        let reward = match ctx.get_final_reward() {
            Some(rew) => rew.clone(),
            None => {
                EsdtTokenPayment::new(TokenIdentifier::from_esdt_bytes(&[]), 0, BigUint::zero())
            }
        };
        let output = ctx.get_output_payments().get(0);
        let output_attributes = ctx
            .get_output_attributes()
            .unwrap_or_else(|| sc_panic!("No farm attributes"));

        self.claim_rewards_event(
            ctx.get_caller(),
            ctx.get_farm_token_id(),
            ctx.get_block_epoch(),
            &ClaimRewardsEvent {
                caller: ctx.get_caller().clone(),
                old_farm_token_id: ctx.get_farm_token_id().clone(),
                old_farm_token_nonce: first_pay.token_nonce,
                old_farm_token_amount: first_pay.amount.clone(),
                new_farm_token_id: ctx.get_farm_token_id().clone(),
                new_farm_token_nonce: output.token_nonce,
                new_farm_token_amount: output.amount,
                farm_supply: ctx.get_farm_token_supply().clone(),
                reward_token_id: reward.token_identifier,
                reward_token_nonce: reward.token_nonce,
                reward_token_amount: reward.amount,
                reward_reserve: ctx.get_reward_reserve().clone(),
                old_farm_attributes: ctx.get_input_attributes().clone(),
                new_farm_attributes: output_attributes.clone(),
                created_with_merge: ctx.was_output_created_with_merge(),
                block: ctx.get_block_nonce(),
                epoch: ctx.get_block_epoch(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_compound_rewards_event(self, ctx: &GenericContext<Self::Api>) {
        let first_pay = &ctx.get_tx_input().first_payment;
        let reward = match ctx.get_final_reward() {
            Some(rew) => rew.clone(),
            None => {
                EsdtTokenPayment::new(TokenIdentifier::from_esdt_bytes(&[]), 0, BigUint::zero())
            }
        };
        let output = ctx.get_output_payments().get(0);
        let output_attributes = ctx
            .get_output_attributes()
            .unwrap_or_else(|| sc_panic!("No farm attributes"));

        self.compound_rewards_event(
            ctx.get_caller(),
            ctx.get_farm_token_id(),
            ctx.get_block_epoch(),
            &CompoundRewardsEvent {
                caller: ctx.get_caller().clone(),
                old_farm_token_id: ctx.get_farm_token_id().clone(),
                old_farm_token_nonce: first_pay.token_nonce,
                old_farm_token_amount: first_pay.amount.clone(),
                new_farm_token_id: ctx.get_farm_token_id().clone(),
                new_farm_token_nonce: output.token_nonce,
                new_farm_token_amount: output.amount,
                farm_supply: ctx.get_farm_token_supply().clone(),
                reward_token_id: reward.token_identifier,
                reward_token_nonce: reward.token_nonce,
                reward_token_amount: reward.amount,
                reward_reserve: ctx.get_reward_reserve().clone(),
                old_farm_attributes: ctx.get_input_attributes().clone(),
                new_farm_attributes: output_attributes.clone(),
                created_with_merge: ctx.was_output_created_with_merge(),
                block: ctx.get_block_nonce(),
                epoch: ctx.get_block_epoch(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }
    */

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
