elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{
    GenericTokenAmountPair, WrappedFarmTokenAttributes, WrappedLpTokenAttributes,
};

#[derive(TopEncode)]
pub struct AddLiquidityProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    pair_address: Address,
    first_token_amount: GenericTokenAmountPair<BigUint>,
    second_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_lp_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<BigUint>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct RemoveLiquidityProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    pair_address: Address,
    wrapped_lp_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<BigUint>,
    first_token_amount: GenericTokenAmountPair<BigUint>,
    second_token_amount: GenericTokenAmountPair<BigUint>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct EnterFarmProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    farm_address: Address,
    farming_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ExitFarmProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    farm_address: Address,
    wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    farming_token_amount: GenericTokenAmountPair<BigUint>,
    reward_token_amount: GenericTokenAmountPair<BigUint>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ClaimRewardsProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    farm_address: Address,
    old_wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    new_wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    reward_token_amount: GenericTokenAmountPair<BigUint>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct CompoundRewardsProxyEvent<BigUint: BigUintApi> {
    caller: Address,
    farm_address: Address,
    old_wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    new_wrapped_farm_token_amount: GenericTokenAmountPair<BigUint>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<BigUint>,
    created_with_merge: bool,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_add_liquidity_proxy_event(
        self,
        caller: Address,
        pair_address: Address,
        first_token_amount: GenericTokenAmountPair<Self::BigUint>,
        second_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_lp_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::BigUint>,
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
        caller: Address,
        pair_address: Address,
        wrapped_lp_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::BigUint>,
        first_token_amount: GenericTokenAmountPair<Self::BigUint>,
        second_token_amount: GenericTokenAmountPair<Self::BigUint>,
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
        caller: Address,
        farm_address: Address,
        farming_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
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
        caller: Address,
        farm_address: Address,
        wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
        farming_token_amount: GenericTokenAmountPair<Self::BigUint>,
        reward_token_amount: GenericTokenAmountPair<Self::BigUint>,
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
        caller: Address,
        farm_address: Address,
        old_wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        new_wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        reward_token_amount: GenericTokenAmountPair<Self::BigUint>,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
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
        caller: Address,
        farm_address: Address,
        old_wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        new_wrapped_farm_token_amount: GenericTokenAmountPair<Self::BigUint>,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::BigUint>,
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
        #[indexed] caller: Address,
        #[indexed] pair_address: Address,
        #[indexed] epoch: u64,
        add_liquidity_proxy_event: AddLiquidityProxyEvent<Self::BigUint>,
    );

    #[event("remove_liquidity_proxy")]
    fn remove_liquidity_proxy_event(
        self,
        #[indexed] first_token: TokenIdentifier,
        #[indexed] second_token: TokenIdentifier,
        #[indexed] caller: Address,
        #[indexed] pair_address: Address,
        #[indexed] epoch: u64,
        remove_liquidity_proxy_event: RemoveLiquidityProxyEvent<Self::BigUint>,
    );

    #[event("enter_farm_proxy")]
    fn enter_farm_proxy_event(
        self,
        #[indexed] farming_token: TokenIdentifier,
        #[indexed] caller: Address,
        #[indexed] farm_address: Address,
        #[indexed] epoch: u64,
        enter_farm_proxy_event: EnterFarmProxyEvent<Self::BigUint>,
    );

    #[event("exit_farm_proxy")]
    fn exit_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: Address,
        #[indexed] farm_address: Address,
        #[indexed] epoch: u64,
        exit_farm_proxy_event: ExitFarmProxyEvent<Self::BigUint>,
    );

    #[event("claim_rewards_farm_proxy")]
    fn claim_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: Address,
        #[indexed] farm_address: Address,
        #[indexed] epoch: u64,
        claim_rewards_farm_proxy_event: ClaimRewardsProxyEvent<Self::BigUint>,
    );

    #[event("compound_rewards_farm_proxy")]
    fn compound_rewards_farm_proxy_event(
        self,
        #[indexed] farm_token: TokenIdentifier,
        #[indexed] caller: Address,
        #[indexed] farm_address: Address,
        #[indexed] epoch: u64,
        compound_rewards_farm_proxy_event: CompoundRewardsProxyEvent<Self::BigUint>,
    );
}
