multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::FarmTokenAttributes;

#[derive(TopEncode)]
pub struct ExitFarmEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farming_token_id: TokenIdentifier<M>,
    farming_token_amount: BigUint<M>,
    farming_reserve: BigUint<M>,
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

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_exit_farm_event(
        self,
        caller: &ManagedAddress,
        farming_token_id: &TokenIdentifier,
        farming_token_amount: &BigUint,
        farming_reserve: &BigUint,
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
            farm_attributes.with_locked_rewards,
            epoch,
            &ExitFarmEvent {
                caller: caller.clone(),
                farming_token_id: farming_token_id.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farming_reserve: farming_reserve.clone(),
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

    #[event("exit_farm")]
    fn exit_farm_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] with_locked_rewards: bool,
        #[indexed] epoch: u64,
        exit_farm_event: &ExitFarmEvent<Self::Api>,
    );

    #[event("burn_tokens")]
    fn burn_tokens_event(
        &self,
        #[indexed] token_id: &TokenIdentifier,
        #[indexed] burned_now: &BigUint,
        #[indexed] burned_total: &BigUint,
    );
}
