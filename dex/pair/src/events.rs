use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::base::Context;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::contexts::swap::SwapContext;

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
    fn emit_swap_event(&self, context: &SwapContext<Self::Api>) {
        let epoch = self.blockchain().get_block_epoch();
        self.swap_event(
            context.get_token_in(),
            context.get_token_out(),
            context.get_caller(),
            epoch,
            &SwapEvent {
                caller: context.get_caller().clone(),
                token_id_in: context.get_token_in().clone(),
                token_amount_in: context.get_final_input_amount().clone(),
                token_id_out: context.get_token_out().clone(),
                token_amount_out: context.get_final_output_amount().clone(),
                fee_amount: context.get_fee_amount().clone(),
                token_in_reserve: context.get_reserve_in().clone(),
                token_out_reserve: context.get_reserve_out().clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_swap_no_fee_and_forward_event(
        &self,
        context: &SwapContext<Self::Api>,
        destination: &ManagedAddress,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.swap_no_fee_and_forward_event(
            &context.get_swap_args().output_token_id,
            context.get_caller(),
            epoch,
            &SwapNoFeeAndForwardEvent {
                caller: context.get_caller().clone(),
                token_id_in: context.get_payment().token_identifier.clone(),
                token_amount_in: context.get_payment().amount.clone(),
                token_id_out: context.get_swap_args().output_token_id.clone(),
                token_amount_out: context.get_final_output_amount().clone(),
                destination: destination.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_add_liquidity_event(&self, context: &AddLiquidityContext<Self::Api>) {
        let epoch = self.blockchain().get_block_epoch();
        self.add_liquidity_event(
            context.get_first_token_id(),
            context.get_second_token_id(),
            context.get_caller(),
            epoch,
            &AddLiquidityEvent {
                caller: context.get_caller().clone(),
                first_token_id: context.get_first_token_id().clone(),
                first_token_amount: context.get_first_amount_optimal().clone(),
                second_token_id: context.get_second_token_id().clone(),
                second_token_amount: context.get_second_amount_optimal().clone(),
                lp_token_id: context.get_lp_token_id().clone(),
                lp_token_amount: context.get_liquidity_added().clone(),
                lp_supply: context.get_lp_token_supply().clone(),
                first_token_reserves: context.get_first_token_reserve().clone(),
                second_token_reserves: context.get_second_token_reserve().clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_remove_liquidity_event(&self, context: &RemoveLiquidityContext<Self::Api>) {
        let epoch = self.blockchain().get_block_epoch();
        self.remove_liquidity_event(
            context.get_first_token_id(),
            context.get_second_token_id(),
            context.get_caller(),
            epoch,
            &RemoveLiquidityEvent {
                caller: context.get_caller().clone(),
                first_token_id: context.get_first_token_id().clone(),
                first_token_amount: context.get_first_token_amount_removed().clone(),
                second_token_id: context.get_second_token_id().clone(),
                second_token_amount: context.get_second_token_amount_removed().clone(),
                lp_token_id: context.get_lp_token_id().clone(),
                lp_token_amount: context.get_lp_token_payment().amount.clone(),
                lp_supply: context.get_lp_token_supply().clone(),
                first_token_reserves: context.get_first_token_reserve().clone(),
                second_token_reserves: context.get_second_token_reserve().clone(),
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
