elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{WrappedFarmTokenAttributes, WrappedLpTokenAttributes};

#[derive(TypeAbi, TopEncode)]
pub struct RemoveLiquidityProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    pair_address: ManagedAddress<M>,
    wrapped_lp_token_id: TokenIdentifier<M>,
    wrapped_lp_token_nonce: u64,
    wrapped_lp_token_amount: BigUint<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    first_token_id: TokenIdentifier<M>,
    first_token_nonce: u64,
    first_token_amount: BigUint<M>,
    second_token_id: TokenIdentifier<M>,
    second_token_nonce: u64,
    second_token_amount: BigUint<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct ExitFarmProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    wrapped_farm_token_id: TokenIdentifier<M>,
    wrapped_farm_token_nonce: u64,
    wrapped_farm_token_amount: BigUint<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    farming_token_id: TokenIdentifier<M>,
    farming_token_nonce: u64,
    farming_token_amount: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_nonce: u64,
    reward_token_amount: BigUint<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_remove_liquidity_proxy_event(
        self,
        caller: &ManagedAddress,
        pair_address: &ManagedAddress,
        wrapped_lp_token_id: &TokenIdentifier,
        wrapped_lp_token_nonce: u64,
        wrapped_lp_token_amount: &BigUint,
        wrapped_lp_attributes: &WrappedLpTokenAttributes<Self::Api>,
        first_token_id: &TokenIdentifier,
        first_token_nonce: u64,
        first_token_amount: &BigUint,
        second_token_id: &TokenIdentifier,
        second_token_nonce: u64,
        second_token_amount: &BigUint,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.remove_liquidity_proxy_event(
            first_token_id,
            second_token_id,
            caller,
            pair_address,
            epoch,
            &RemoveLiquidityProxyEvent {
                caller: caller.clone(),
                pair_address: pair_address.clone(),
                first_token_id: first_token_id.clone(),
                first_token_nonce,
                first_token_amount: first_token_amount.clone(),
                second_token_id: second_token_id.clone(),
                second_token_nonce,
                second_token_amount: second_token_amount.clone(),
                wrapped_lp_token_id: wrapped_lp_token_id.clone(),
                wrapped_lp_token_nonce,
                wrapped_lp_token_amount: wrapped_lp_token_amount.clone(),
                wrapped_lp_attributes: wrapped_lp_attributes.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_exit_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        wrapped_farm_token_id: &TokenIdentifier,
        wrapped_farm_token_nonce: u64,
        wrapped_farm_token_amount: &BigUint,
        wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        farming_token_id: &TokenIdentifier,
        farming_token_nonce: u64,
        farming_token_amount: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_nonce: u64,
        reward_token_amount: &BigUint,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.exit_farm_proxy_event(
            farming_token_id,
            caller,
            farm_address,
            epoch,
            &ExitFarmProxyEvent {
                caller: caller.clone(),
                farm_address: farm_address.clone(),
                farming_token_id: farming_token_id.clone(),
                farming_token_nonce,
                farming_token_amount: farming_token_amount.clone(),
                wrapped_farm_token_id: wrapped_farm_token_id.clone(),
                wrapped_farm_token_nonce,
                wrapped_farm_token_amount: wrapped_farm_token_amount.clone(),
                wrapped_farm_attributes: wrapped_farm_attributes.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_nonce,
                reward_token_amount: reward_token_amount.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("remove_liquidity_proxy")]
    fn remove_liquidity_proxy_event(
        self,
        #[indexed] first_token: &TokenIdentifier,
        #[indexed] second_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] pair_address: &ManagedAddress,
        #[indexed] epoch: u64,
        remove_liquidity_proxy_event: &RemoveLiquidityProxyEvent<Self::Api>,
    );

    #[event("exit_farm_proxy")]
    fn exit_farm_proxy_event(
        self,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        exit_farm_proxy_event: &ExitFarmProxyEvent<Self::Api>,
    );
}
