use crate::phase::Phase;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode)]
pub struct ExtraRewardsEvent<M: ManagedTypeApi> {
    rewards_token_id: TokenIdentifier<M>,
    rewards_amount: BigUint<M>,
}

#[derive(TopEncode)]
pub struct DepositEvent<M: ManagedTypeApi> {
    token_id_in: TokenIdentifier<M>,
    token_amount_in: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase<M>,
}

#[derive(TopEncode)]
pub struct WithdrawEvent<M: ManagedTypeApi> {
    token_id_out: TokenIdentifier<M>,
    token_amount_out: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase<M>,
}

#[derive(TopEncode)]
pub struct RedeemEvent<M: ManagedTypeApi> {
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    lp_token_id: TokenIdentifier<M>,
    lp_token_amout: BigUint<M>,
    lp_tokens_remaining: BigUint<M>,
    total_lp_tokens_received: BigUint<M>,
    rewards_token_id: TokenIdentifier<M>,
    rewards_token_amount: BigUint<M>,
}

#[derive(TopEncode)]
pub struct InitialLiquidityEvent<M: ManagedTypeApi> {
    lp_token_id: TokenIdentifier<M>,
    lp_tokens_received: BigUint<M>,
}

#[elrond_wasm::module]
pub trait EventsModule: crate::common_storage::CommonStorageModule {
    fn emit_deposit_extra_rewards_event(
        &self,
        rewards_token_id: TokenIdentifier,
        rewards_amount: BigUint,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        self.deposit_extra_rewards_event(
            &caller,
            block,
            epoch,
            timestamp,
            &ExtraRewardsEvent {
                rewards_token_id,
                rewards_amount,
            },
        );
    }

    fn emit_deposit_event(
        &self,
        token_id_in: TokenIdentifier,
        token_amount_in: BigUint,
        redeem_token_id: TokenIdentifier,
        redeem_token_nonce: u64,
        redeem_token_amount: BigUint,
        current_price: BigUint,
        current_phase: Phase<Self::Api>,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        let launched_token_amount = self.launched_token_balance().get();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.deposit_event(
            &caller,
            block,
            epoch,
            timestamp,
            &DepositEvent {
                token_id_in,
                token_amount_in,
                redeem_token_id,
                redeem_token_nonce,
                redeem_token_amount,
                launched_token_amount,
                accepted_token_amount,
                current_price,
                current_phase,
            },
        );
    }

    fn emit_withdraw_event(
        &self,
        token_id_out: TokenIdentifier,
        token_amount_out: BigUint,
        redeem_token_id: TokenIdentifier,
        redeem_token_nonce: u64,
        redeem_token_amount: BigUint,
        current_price: BigUint,
        current_phase: Phase<Self::Api>,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        let launched_token_amount = self.launched_token_balance().get();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.withdraw_event(
            &caller,
            block,
            epoch,
            timestamp,
            &WithdrawEvent {
                token_id_out,
                token_amount_out,
                redeem_token_id,
                redeem_token_nonce,
                redeem_token_amount,
                launched_token_amount,
                accepted_token_amount,
                current_price,
                current_phase,
            },
        );
    }

    fn emit_redeem_event(
        &self,
        redeem_token_id: TokenIdentifier,
        redeem_token_nonce: u64,
        redeem_token_amount: BigUint,
        lp_token_id: TokenIdentifier,
        lp_token_amout: BigUint,
        lp_tokens_remaining: BigUint,
        total_lp_tokens_received: BigUint,
        rewards_token_id: TokenIdentifier,
        rewards_token_amount: BigUint,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        self.redeem_event(
            &caller,
            block,
            epoch,
            timestamp,
            &RedeemEvent {
                redeem_token_id,
                redeem_token_nonce,
                redeem_token_amount,
                lp_token_id,
                lp_token_amout,
                lp_tokens_remaining,
                total_lp_tokens_received,
                rewards_token_id,
                rewards_token_amount,
            },
        )
    }

    fn emit_initial_liquidity_event(
        &self,
        lp_token_id: TokenIdentifier,
        lp_tokens_received: BigUint,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        self.liquidity_pool_created_event(
            &caller,
            block,
            epoch,
            timestamp,
            &InitialLiquidityEvent {
                lp_token_id,
                lp_tokens_received,
            },
        )
    }

    #[event("depositExtraRewardsEvent")]
    fn deposit_extra_rewards_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        extra_rewards_event: &ExtraRewardsEvent<Self::Api>,
    );

    #[event("depositEvent")]
    fn deposit_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        deposit_event: &DepositEvent<Self::Api>,
    );

    #[event("withdrawEvent")]
    fn withdraw_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        withdraw_event: &WithdrawEvent<Self::Api>,
    );

    #[event("liquidityPoolCreatedEvent")]
    fn liquidity_pool_created_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        initial_liquidity_event: &InitialLiquidityEvent<Self::Api>,
    );

    #[event("redeemEvent")]
    fn redeem_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        redeem_event: &RedeemEvent<Self::Api>,
    );
}
