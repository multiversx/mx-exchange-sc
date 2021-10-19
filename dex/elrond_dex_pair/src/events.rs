elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode)]
pub struct SwapEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    token_id_in: TokenIdentifier<M>,
    token_amount_in: BigUint<M>,
    token_id_out: TokenIdentifier<M>,
    token_amount_out: BigUint<M>,
    fee_amount: BigUint<M>,
    token_in_reserve: BigUint<M>,
    token_out_reserve: BigUint<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct SwapNoFeeAndForwardEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    token_id_in: TokenIdentifier<M>,
    token_amount_in: BigUint<M>,
    token_id_out: TokenIdentifier<M>,
    token_amount_out: BigUint<M>,
    destination: ManagedAddress<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct AddLiquidityEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    first_token_id: TokenIdentifier<M>,
    first_token_amount: BigUint<M>,
    second_token_id: TokenIdentifier<M>,
    second_token_amount: BigUint<M>,
    lp_token_id: TokenIdentifier<M>,
    lp_token_amount: BigUint<M>,
    lp_supply: BigUint<M>,
    first_token_reserves: BigUint<M>,
    second_token_reserves: BigUint<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct RemoveLiquidityEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    first_token_id: TokenIdentifier<M>,
    first_token_amount: BigUint<M>,
    second_token_id: TokenIdentifier<M>,
    second_token_amount: BigUint<M>,
    lp_token_id: TokenIdentifier<M>,
    lp_token_amount: BigUint<M>,
    lp_supply: BigUint<M>,
    first_token_reserves: BigUint<M>,
    second_token_reserves: BigUint<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_swap_event(
        &self,
        caller: &ManagedAddress,
        token_id_in: &TokenIdentifier,
        token_amount_in: &BigUint,
        token_id_out: &TokenIdentifier,
        token_amount_out: &BigUint,
        fee_amount: &BigUint,
        token_in_reserve: &BigUint,
        token_out_reserve: &BigUint,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.swap_event(
            token_id_in,
            token_id_out,
            caller,
            epoch,
            &SwapEvent {
                caller: caller.clone(),
                token_id_in: token_id_in.clone(),
                token_amount_in: token_amount_in.clone(),
                token_id_out: token_id_out.clone(),
                token_amount_out: token_amount_out.clone(),
                fee_amount: fee_amount.clone(),
                token_in_reserve: token_in_reserve.clone(),
                token_out_reserve: token_out_reserve.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_swap_no_fee_and_forward_event(
        &self,
        caller: &ManagedAddress,
        token_id_in: &TokenIdentifier,
        token_amount_in: &BigUint,
        token_id_out: &TokenIdentifier,
        token_amount_out: &BigUint,
        destination: &ManagedAddress,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.swap_no_fee_and_forward_event(
            token_id_out,
            caller,
            epoch,
            &SwapNoFeeAndForwardEvent {
                caller: caller.clone(),
                token_id_in: token_id_in.clone(),
                token_amount_in: token_amount_in.clone(),
                token_id_out: token_id_out.clone(),
                token_amount_out: token_amount_out.clone(),
                destination: destination.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_add_liquidity_event(
        &self,
        caller: &ManagedAddress,
        first_token_id: &TokenIdentifier,
        first_token_amount: &BigUint,
        second_token_id: &TokenIdentifier,
        second_token_amount: &BigUint,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &BigUint,
        lp_supply: &BigUint,
        first_token_reserves: &BigUint,
        second_token_reserves: &BigUint,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.add_liquidity_event(
            first_token_id,
            second_token_id,
            caller,
            epoch,
            &AddLiquidityEvent {
                caller: caller.clone(),
                first_token_id: first_token_id.clone(),
                first_token_amount: first_token_amount.clone(),
                second_token_id: second_token_id.clone(),
                second_token_amount: second_token_amount.clone(),
                lp_token_id: lp_token_id.clone(),
                lp_token_amount: lp_token_amount.clone(),
                lp_supply: lp_supply.clone(),
                first_token_reserves: first_token_reserves.clone(),
                second_token_reserves: second_token_reserves.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_remove_liquidity_event(
        &self,
        caller: &ManagedAddress,
        first_token_id: &TokenIdentifier,
        first_token_amount: &BigUint,
        second_token_id: &TokenIdentifier,
        second_token_amount: &BigUint,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &BigUint,
        lp_supply: &BigUint,
        first_token_reserves: &BigUint,
        second_token_reserves: &BigUint,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.remove_liquidity_event(
            first_token_id,
            second_token_id,
            caller,
            epoch,
            &RemoveLiquidityEvent {
                caller: caller.clone(),
                first_token_id: first_token_id.clone(),
                first_token_amount: first_token_amount.clone(),
                second_token_id: second_token_id.clone(),
                second_token_amount: second_token_amount.clone(),
                lp_token_id: lp_token_id.clone(),
                lp_token_amount: lp_token_amount.clone(),
                lp_supply: lp_supply.clone(),
                first_token_reserves: first_token_reserves.clone(),
                second_token_reserves: second_token_reserves.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("swap")]
    fn swap_event(
        &self,
        #[indexed] token_in: &TokenIdentifier,
        #[indexed] token_out: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        swap_event: &SwapEvent<Self::Api>,
    );

    #[event("swap_no_fee_and_forward")]
    fn swap_no_fee_and_forward_event(
        &self,
        #[indexed] token_id_out: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        swap_no_fee_and_forward_event: &SwapNoFeeAndForwardEvent<Self::Api>,
    );

    #[event("add_liquidity")]
    fn add_liquidity_event(
        &self,
        #[indexed] first_token: &TokenIdentifier,
        #[indexed] second_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        add_liquidity_event: &AddLiquidityEvent<Self::Api>,
    );

    #[event("remove_liquidity")]
    fn remove_liquidity_event(
        &self,
        #[indexed] first_token: &TokenIdentifier,
        #[indexed] second_token: &TokenIdentifier,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        remove_liquidity_event: &RemoveLiquidityEvent<Self::Api>,
    );
}
