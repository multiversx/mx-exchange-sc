multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

#[derive(TypeAbi, TopEncode)]
pub struct AddLiquidityProxyEvent<M: ManagedTypeApi> {
    first_token: EsdtTokenPayment<M>,
    second_token: EsdtTokenPayment<M>,
    wrapped_lp_token: EsdtTokenPayment<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    created_with_merge: bool,
}

#[derive(TypeAbi, TopEncode)]
pub struct RemoveLiquidityProxyEvent<M: ManagedTypeApi> {
    wrapped_lp_token: EsdtTokenPayment<M>,
    wrapped_lp_attributes: WrappedLpTokenAttributes<M>,
    first_token: EsdtTokenPayment<M>,
    second_token: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct EnterFarmProxyEvent<M: ManagedTypeApi> {
    farming_token: EsdtTokenPayment<M>,
    wrapped_farm_token: EsdtTokenPayment<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    created_with_merge: bool,
}

#[derive(TypeAbi, TopEncode)]
pub struct ExitFarmProxyEvent<M: ManagedTypeApi> {
    wrapped_farm_token: EsdtTokenPayment<M>,
    wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    reward_tokens: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct ClaimRewardsProxyEvent<M: ManagedTypeApi> {
    old_wrapped_farm_token: EsdtTokenPayment<M>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    new_wrapped_farm_token: EsdtTokenPayment<M>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    reward_tokens: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct CompoundRewardsProxyEvent<M: ManagedTypeApi> {
    old_wrapped_farm_token: EsdtTokenPayment<M>,
    old_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
    new_wrapped_farm_token: EsdtTokenPayment<M>,
    new_wrapped_farm_attributes: WrappedFarmTokenAttributes<M>,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_add_liquidity_proxy_event(
        self,
        caller: &ManagedAddress,
        pair_address: &ManagedAddress,
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
            caller,
            pair_address,
            epoch,
            block,
            timestamp,
            &AddLiquidityProxyEvent {
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
        caller: &ManagedAddress,
        pair_address: &ManagedAddress,
        wrapped_lp_token: EsdtTokenPayment,
        wrapped_lp_attributes: WrappedLpTokenAttributes<Self::Api>,
        first_token: EsdtTokenPayment,
        second_token: EsdtTokenPayment,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.remove_liquidity_proxy_event(
            caller,
            pair_address,
            epoch,
            block,
            timestamp,
            &RemoveLiquidityProxyEvent {
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
        farming_token: EsdtTokenPayment,
        wrapped_farm_token: EsdtTokenPayment,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        created_with_merge: bool,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.enter_farm_proxy_event(
            caller,
            farm_address,
            epoch,
            block,
            timestamp,
            &EnterFarmProxyEvent {
                farming_token,
                wrapped_farm_token,
                wrapped_farm_attributes,
                created_with_merge,
            },
        )
    }

    fn emit_exit_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        wrapped_farm_token: EsdtTokenPayment,
        wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        reward_tokens: EsdtTokenPayment,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.exit_farm_proxy_event(
            caller,
            farm_address,
            epoch,
            block,
            timestamp,
            &ExitFarmProxyEvent {
                wrapped_farm_token,
                wrapped_farm_attributes,
                reward_tokens,
            },
        )
    }

    fn emit_claim_rewards_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        old_wrapped_farm_token: EsdtTokenPayment,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_token: EsdtTokenPayment,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        reward_tokens: EsdtTokenPayment,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.claim_rewards_farm_proxy_event(
            caller,
            farm_address,
            epoch,
            block,
            timestamp,
            &ClaimRewardsProxyEvent {
                old_wrapped_farm_token,
                old_wrapped_farm_attributes,
                new_wrapped_farm_token,
                new_wrapped_farm_attributes,
                reward_tokens,
            },
        )
    }

    fn emit_compound_rewards_farm_proxy_event(
        self,
        caller: &ManagedAddress,
        farm_address: &ManagedAddress,
        old_wrapped_farm_token: EsdtTokenPayment,
        old_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
        new_wrapped_farm_token: EsdtTokenPayment,
        new_wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let timestamp = self.blockchain().get_block_timestamp();
        self.compound_rewards_farm_proxy_event(
            caller,
            farm_address,
            epoch,
            block,
            timestamp,
            &CompoundRewardsProxyEvent {
                old_wrapped_farm_token,
                old_wrapped_farm_attributes,
                new_wrapped_farm_token,
                new_wrapped_farm_attributes,
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
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        enter_farm_proxy_event: &EnterFarmProxyEvent<Self::Api>,
    );

    #[event("exit_farm_proxy")]
    fn exit_farm_proxy_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        exit_farm_proxy_event: &ExitFarmProxyEvent<Self::Api>,
    );

    #[event("claim_rewards_farm_proxy")]
    fn claim_rewards_farm_proxy_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        claim_rewards_farm_proxy_event: &ClaimRewardsProxyEvent<Self::Api>,
    );

    #[event("compound_rewards_farm_proxy")]
    fn compound_rewards_farm_proxy_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] farm_address: &ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] block: u64,
        #[indexed] timestamp: u64,
        compound_rewards_farm_proxy_event: &CompoundRewardsProxyEvent<Self::Api>,
    );
}
