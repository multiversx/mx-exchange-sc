elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

#[derive(TypeAbi, TopEncode)]
pub struct AddLiquidityProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    pair_address: ManagedAddress<M>,
    first_token: EsdtTokenPayment<M>,
    second_token: EsdtTokenPayment<M>,
    wrapped_lp_token: EsdtTokenPayment<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    created_with_merge: bool,
}

#[derive(TypeAbi, TopEncode)]
pub struct RemoveLiquidityProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    pair_address: ManagedAddress<M>,
    wrapped_lp_token: EsdtTokenPayment<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    first_token: EsdtTokenPayment<M>,
    second_token: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct EnterFarmProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    farming_token_id: TokenIdentifier<M>,
    farming_token_nonce: u64,
    farming_token_amount: BigUint<M>,
    wrapped_farm_token_id: TokenIdentifier<M>,
    wrapped_farm_token_nonce: u64,
    wrapped_farm_token_amount: BigUint<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
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

#[derive(TypeAbi, TopEncode)]
pub struct ClaimRewardsProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    old_wrapped_farm_token_id: TokenIdentifier<M>,
    old_wrapped_farm_token_nonce: u64,
    old_wrapped_farm_token_amount: BigUint<M>,
    new_wrapped_farm_token_id: TokenIdentifier<M>,
    new_wrapped_farm_token_nonce: u64,
    new_wrapped_farm_token_amount: BigUint<M>,
    reward_token_id: TokenIdentifier<M>,
    reward_token_nonce: u64,
    reward_token_amount: BigUint<M>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct CompoundRewardsProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    old_wrapped_farm_token_id: TokenIdentifier<M>,
    old_wrapped_farm_token_nonce: u64,
    old_wrapped_farm_token_amount: BigUint<M>,
    new_wrapped_farm_token_id: TokenIdentifier<M>,
    new_wrapped_farm_token_nonce: u64,
    new_wrapped_farm_token_amount: BigUint<M>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_add_liquidity_proxy_event(
        self,
        caller: ManagedAddress,
        pair_address: ManagedAddress,
        first_token: EsdtTokenPayment,
        second_token: EsdtTokenPayment,
        wrapped_lp_token: EsdtTokenPayment,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.add_liquidity_proxy_event(
            &caller.clone(),
            &pair_address.clone(),
            epoch,
            block,
            timestamp,
            &AddLiquidityProxyEvent {
                caller,
                pair_address,
                first_token,
                second_token,
                wrapped_lp_token,
                wrapped_lp_attributes,
                created_with_merge,
            },
        )
    }

    fn emit_remove_liquidity_proxy_event(
        self,
        caller: ManagedAddress,
        pair_address: ManagedAddress,
        wrapped_lp_token: EsdtTokenPayment,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api>,
        first_token: EsdtTokenPayment,
        second_token: EsdtTokenPayment,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.remove_liquidity_proxy_event(
            &caller.clone(),
            &pair_address.clone(),
            epoch,
            block,
            timestamp,
            &RemoveLiquidityProxyEvent {
                caller,
                pair_address,
                wrapped_lp_token,
                wrapped_lp_attributes,
                first_token,
                second_token,
            },
        )
    }

    fn emit_enter_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        farming_token_id: &TokenIdentifier,
        farming_token_nonce: u64,
        farming_token_amount: &BigUint,
        wrapped_farm_token_id: &TokenIdentifier,
        wrapped_farm_token_nonce: u64,
        wrapped_farm_token_amount: &BigUint,
        wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.enter_farm_proxy_event(
            farming_token_id,
            caller,
            farm_address,
            epoch,
            &EnterFarmProxyEvent {
                caller: caller.clone(),
                farm_address: farm_address.clone(),
                farming_token_id: farming_token_id.clone(),
                farming_token_nonce,
                farming_token_amount: farming_token_amount.clone(),
                wrapped_farm_token_id: wrapped_farm_token_id.clone(),
                wrapped_farm_token_nonce,
                wrapped_farm_token_amount: wrapped_farm_token_amount.clone(),
                wrapped_farm_attributes: wrapped_farm_attributes.clone(),
                created_with_merge,
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

    fn emit_claim_rewards_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        old_wrapped_farm_token_id: &TokenIdentifier,
        old_wrapped_farm_token_nonce: u64,
        old_wrapped_farm_token_amount: &BigUint,
        new_wrapped_farm_token_id: &TokenIdentifier,
        new_wrapped_farm_token_nonce: u64,
        new_wrapped_farm_token_amount: &BigUint,
        reward_token_id: &TokenIdentifier,
        reward_token_nonce: u64,
        reward_token_amount: &BigUint,
        old_wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.claim_rewards_farm_proxy_event(
            old_wrapped_farm_token_id,
            caller,
            farm_address,
            epoch,
            &ClaimRewardsProxyEvent {
                caller: caller.clone(),
                farm_address: farm_address.clone(),
                old_wrapped_farm_token_id: old_wrapped_farm_token_id.clone(),
                old_wrapped_farm_token_nonce,
                old_wrapped_farm_token_amount: old_wrapped_farm_token_amount.clone(),
                new_wrapped_farm_token_id: new_wrapped_farm_token_id.clone(),
                new_wrapped_farm_token_nonce,
                new_wrapped_farm_token_amount: new_wrapped_farm_token_amount.clone(),
                reward_token_id: reward_token_id.clone(),
                reward_token_nonce,
                reward_token_amount: reward_token_amount.clone(),
                old_wrapped_farm_attributes: old_wrapped_farm_attributes.clone(),
                new_wrapped_farm_attributes: new_wrapped_farm_attributes.clone(),
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_compound_rewards_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        old_wrapped_farm_token_id: &TokenIdentifier,
        old_wrapped_farm_token_nonce: u64,
        old_wrapped_farm_token_amount: &BigUint,
        new_wrapped_farm_token_id: &TokenIdentifier,
        new_wrapped_farm_token_nonce: u64,
        new_wrapped_farm_token_amount: &BigUint,
        old_wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_attributes: &WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.compound_rewards_farm_proxy_event(
            old_wrapped_farm_token_id,
            caller,
            farm_address,
            epoch,
            &CompoundRewardsProxyEvent {
                caller: caller.clone(),
                farm_address: farm_address.clone(),
                old_wrapped_farm_token_id: old_wrapped_farm_token_id.clone(),
                old_wrapped_farm_token_nonce,
                old_wrapped_farm_token_amount: old_wrapped_farm_token_amount.clone(),
                new_wrapped_farm_token_id: new_wrapped_farm_token_id.clone(),
                new_wrapped_farm_token_nonce,
                new_wrapped_farm_token_amount: new_wrapped_farm_token_amount.clone(),
                old_wrapped_farm_attributes: old_wrapped_farm_attributes.clone(),
                new_wrapped_farm_attributes: new_wrapped_farm_attributes.clone(),
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("add_liquidity_proxy")]
    fn add_liquidity_proxy_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] pair_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        add_liquidity_proxy_event: &AddLiquidityProxyEvent<Self::Api>,
    );

    #[event("remove_liquidity_proxy")]
    fn remove_liquidity_proxy_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] pair_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        remove_liquidity_proxy_event: &RemoveLiquidityProxyEvent<Self::Api>,
    );

    #[event("enter_farm_proxy")]
    fn enter_farm_proxy_event(
        self,
        #[indexed] farming_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        enter_farm_proxy_event: &EnterFarmProxyEvent<Self::Api>,
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

    #[event("claim_rewards_farm_proxy")]
    fn claim_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        claim_rewards_farm_proxy_event: &ClaimRewardsProxyEvent<Self::Api>,
    );

    #[event("compound_rewards_farm_proxy")]
    fn compound_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        compound_rewards_farm_proxy_event: &CompoundRewardsProxyEvent<Self::Api>,
    );
}
