#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;

#[derive(TopEncode)]
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

#[derive(TopEncode)]
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

#[derive(TopEncode)]
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

#[derive(TopEncode)]
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
    fn emit_enter_farm_event(
        self,
        caller: &ManagedAddress,
        farming_token_id: &TokenIdentifier,
        farming_token_amount: &BigUint,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: &BigUint,
        farm_supply: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_reserve: &BigUint,
        farm_attributes: &FarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.enter_farm_event(
            caller,
            farm_token_id,
            epoch,
            &EnterFarmEvent {
                caller: caller.clone(),
                farming_token_id: farming_token_id.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farm_token_id: farm_token_id.clone(),
                farm_token_nonce,
                farm_token_amount: farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_reserve: reward_token_reserve.clone(),
                farm_attributes: farm_attributes.clone(),
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_exit_farm_event(
        self,
        caller: &ManagedAddress,
        farming_token_id: &TokenIdentifier,
        farming_token_amount: &BigUint,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: &BigUint,
        farm_supply: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_nonce: u64,
        reward_token_amount: &BigUint,
        reward_reserve: &BigUint,
        farm_attributes: &FarmTokenAttributes<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.exit_farm_event(
            caller,
            farm_token_id,
            epoch,
            &ExitFarmEvent {
                caller: caller.clone(),
                farming_token_id: farming_token_id.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farm_token_id: farm_token_id.clone(),
                farm_token_nonce,
                farm_token_amount: farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_nonce,
                reward_token_amount: reward_token_amount.clone(),
                reward_reserve: reward_reserve.clone(),
                farm_attributes: farm_attributes.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_claim_rewards_event(
        self,
        caller: &ManagedAddress,
        old_farm_token_id: &TokenIdentifier,
        old_farm_token_nonce: u64,
        old_farm_token_amount: &BigUint,
        new_farm_token_id: &TokenIdentifier,
        new_farm_token_nonce: u64,
        new_farm_token_amount: &BigUint,
        farm_supply: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_nonce: u64,
        reward_token_amount: &BigUint,
        reward_reserve: &BigUint,
        old_farm_attributes: &FarmTokenAttributes<Self::Api>,
        new_farm_attributes: &FarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.claim_rewards_event(
            caller,
            old_farm_token_id,
            epoch,
            &ClaimRewardsEvent {
                caller: caller.clone(),
                old_farm_token_id: old_farm_token_id.clone(),
                old_farm_token_nonce,
                old_farm_token_amount: old_farm_token_amount.clone(),
                new_farm_token_id: new_farm_token_id.clone(),
                new_farm_token_nonce,
                new_farm_token_amount: new_farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_nonce,
                reward_token_amount: reward_token_amount.clone(),
                reward_reserve: reward_reserve.clone(),
                old_farm_attributes: old_farm_attributes.clone(),
                new_farm_attributes: new_farm_attributes.clone(),
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_compound_rewards_event(
        self,
        caller: &ManagedAddress,
        old_farm_token_id: &TokenIdentifier,
        old_farm_token_nonce: u64,
        old_farm_token_amount: &BigUint,
        new_farm_token_id: &TokenIdentifier,
        new_farm_token_nonce: u64,
        new_farm_token_amount: &BigUint,
        farm_supply: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_nonce: u64,
        reward_token_amount: &BigUint,
        reward_reserve: &BigUint,
        old_farm_attributes: &FarmTokenAttributes<Self::Api>,
        new_farm_attributes: &FarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.compound_rewards_event(
            caller,
            old_farm_token_id,
            epoch,
            &CompoundRewardsEvent {
                caller: caller.clone(),
                old_farm_token_id: old_farm_token_id.clone(),
                old_farm_token_nonce,
                old_farm_token_amount: old_farm_token_amount.clone(),
                new_farm_token_id: new_farm_token_id.clone(),
                new_farm_token_nonce,
                new_farm_token_amount: new_farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_nonce,
                reward_token_amount: reward_token_amount.clone(),
                reward_reserve: reward_reserve.clone(),
                old_farm_attributes: old_farm_attributes.clone(),
                new_farm_attributes: new_farm_attributes.clone(),
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("enter_farm")]
    fn enter_farm_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farming_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        enter_farm_event: &EnterFarmEvent<Self::Api>,
    );

    #[event("exit_farm")]
    fn exit_farm_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] epoch: u64,
        exit_farm_event: &ExitFarmEvent<Self::Api>,
    );

    #[event("claim_rewards")]
    fn claim_rewards_event(
        self,
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
