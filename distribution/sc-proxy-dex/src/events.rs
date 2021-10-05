elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{
    GenericTokenAmountPair, WrappedFarmTokenAttributes, WrappedLpTokenAttributes,
};

#[derive(TopEncode)]
pub struct AddLiquidityProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    pair_address: ManagedAddress<M>,
    first_token_amount: GenericTokenAmountPair<M>,
    second_token_amount: GenericTokenAmountPair<M>,
    wrapped_lp_token_amount: GenericTokenAmountPair<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct RemoveLiquidityProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    pair_address: ManagedAddress<M>,
    wrapped_lp_token_amount: GenericTokenAmountPair<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    first_token_amount: GenericTokenAmountPair<M>,
    second_token_amount: GenericTokenAmountPair<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct EnterFarmProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    farming_token_amount: GenericTokenAmountPair<M>,
    wrapped_farm_token_amount: GenericTokenAmountPair<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ExitFarmProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    wrapped_farm_token_amount: GenericTokenAmountPair<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    farming_token_amount: GenericTokenAmountPair<M>,
    reward_token_amount: GenericTokenAmountPair<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ClaimRewardsProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    old_wrapped_farm_token_amount: GenericTokenAmountPair<M>,
    new_wrapped_farm_token_amount: GenericTokenAmountPair<M>,
    reward_token_amount: GenericTokenAmountPair<M>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct CompoundRewardsProxyEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    old_wrapped_farm_token_amount: GenericTokenAmountPair<M>,
    new_wrapped_farm_token_amount: GenericTokenAmountPair<M>,
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
        first_token_amount: GenericTokenAmountPair<Self::Api>,
        second_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_lp_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.add_liquidity_proxy_event(
            first_token_amount.token_id.clone(),
            second_token_amount.token_id.clone(),
            caller.clone(),
            pair_address.clone(),
            epoch,
            AddLiquidityProxyEvent {
                caller,
                pair_address,
                first_token_amount,
                second_token_amount,
                wrapped_lp_token_amount,
                wrapped_lp_attributes,
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_remove_liquidity_proxy_event(
        self,
        caller: ManagedAddress,
        pair_address: ManagedAddress,
        wrapped_lp_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api>,
        first_token_amount: GenericTokenAmountPair<Self::Api>,
        second_token_amount: GenericTokenAmountPair<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.remove_liquidity_proxy_event(
            first_token_amount.token_id.clone(),
            second_token_amount.token_id.clone(),
            caller.clone(),
            pair_address.clone(),
            epoch,
            RemoveLiquidityProxyEvent {
                caller,
                pair_address,
                wrapped_lp_token_amount,
                wrapped_lp_attributes,
                first_token_amount,
                second_token_amount,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_enter_farm_proxy_event(
        self,
        caller: ManagedAddress,
        farm_address: ManagedAddress,
        farming_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.enter_farm_proxy_event(
            farming_token_amount.token_id.clone(),
            caller.clone(),
            farm_address.clone(),
            epoch,
            EnterFarmProxyEvent {
                caller,
                farm_address,
                farming_token_amount,
                wrapped_farm_token_amount,
                wrapped_farm_attributes,
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_exit_farm_proxy_event(
        self,
        caller: ManagedAddress,
        farm_address: ManagedAddress,
        wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        farming_token_amount: GenericTokenAmountPair<Self::Api>,
        reward_token_amount: GenericTokenAmountPair<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.exit_farm_proxy_event(
            farming_token_amount.token_id.clone(),
            caller.clone(),
            farm_address.clone(),
            epoch,
            ExitFarmProxyEvent {
                caller,
                farm_address,
                farming_token_amount,
                wrapped_farm_token_amount,
                wrapped_farm_attributes,
                reward_token_amount,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_claim_rewards_farm_proxy_event(
        self,
        caller: ManagedAddress,
        farm_address: ManagedAddress,
        old_wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        new_wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        reward_token_amount: GenericTokenAmountPair<Self::Api>,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.claim_rewards_farm_proxy_event(
            old_wrapped_farm_token_amount.token_id.clone(),
            caller.clone(),
            farm_address.clone(),
            epoch,
            ClaimRewardsProxyEvent {
                caller,
                farm_address,
                old_wrapped_farm_token_amount,
                new_wrapped_farm_token_amount,
                reward_token_amount,
                old_wrapped_farm_attributes,
                new_wrapped_farm_attributes,
                created_with_merge,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_compound_rewards_farm_proxy_event(
        self,
        caller: ManagedAddress,
        farm_address: ManagedAddress,
        old_wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        new_wrapped_farm_token_amount: GenericTokenAmountPair<Self::Api>,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.compound_rewards_farm_proxy_event(
            old_wrapped_farm_token_amount.token_id.clone(),
            caller.clone(),
            farm_address.clone(),
            epoch,
            CompoundRewardsProxyEvent {
                caller,
                farm_address,
                old_wrapped_farm_token_amount,
                new_wrapped_farm_token_amount,
                old_wrapped_farm_attributes,
                new_wrapped_farm_attributes,
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
        #[indexed] first_token: TokenIdentifier,
        #[indexed] second_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] pair_address: ManagedAddress,
        #[indexed] epoch: u64,
        add_liquidity_proxy_event: AddLiquidityProxyEvent<Self::Api>,
    );

    #[event("remove_liquidity_proxy")]
    fn remove_liquidity_proxy_event(
        self,
        #[indexed] first_token: TokenIdentifier,
        #[indexed] second_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] pair_address: ManagedAddress,
        #[indexed] epoch: u64,
        remove_liquidity_proxy_event: RemoveLiquidityProxyEvent<Self::Api>,
    );

    #[event("enter_farm_proxy")]
    fn enter_farm_proxy_event(
        self,
        #[indexed] farming_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] farm_address: ManagedAddress,
        #[indexed] epoch: u64,
        enter_farm_proxy_event: EnterFarmProxyEvent<Self::Api>,
    );

    #[event("exit_farm_proxy")]
    fn exit_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] farm_address: ManagedAddress,
        #[indexed] epoch: u64,
        exit_farm_proxy_event: ExitFarmProxyEvent<Self::Api>,
    );

    #[event("claim_rewards_farm_proxy")]
    fn claim_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] farm_address: ManagedAddress,
        #[indexed] epoch: u64,
        claim_rewards_farm_proxy_event: ClaimRewardsProxyEvent<Self::Api>,
    );

    #[event("compound_rewards_farm_proxy")]
    fn compound_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] farm_address: ManagedAddress,
        #[indexed] epoch: u64,
        compound_rewards_farm_proxy_event: CompoundRewardsProxyEvent<Self::Api>,
    );
}
