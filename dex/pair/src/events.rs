use crate::contexts::add_liquidity::AddLiquidityContext;
use crate::contexts::base::StorageCache;
use crate::contexts::remove_liquidity::RemoveLiquidityContext;
use crate::contexts::swap::SwapContext;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode)]
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

#[derive(TypeAbi, TopEncode)]
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

#[derive(TypeAbi, TopEncode)]
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

#[derive(TypeAbi, TopEncode)]
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

#[multiversx_sc::module]
pub trait EventsModule:
    crate::config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn emit_swap_event(&self, storage_cache: &StorageCache<Self>, context: SwapContext<Self::Api>) {
        let epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        self.swap_event(
            &context.input_token_id.clone(),
            &context.output_token_id.clone(),
            &caller,
            epoch,
            &SwapEvent {
                caller: caller.clone(),
                token_id_in: context.input_token_id,
                token_amount_in: context.final_input_amount,
                token_id_out: context.output_token_id,
                token_amount_out: context.final_output_amount,
                fee_amount: context.fee_amount,
                token_in_reserve: storage_cache
                    .get_reserve_in(context.swap_tokens_order)
                    .clone(),
                token_out_reserve: storage_cache
                    .get_reserve_out(context.swap_tokens_order)
                    .clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_swap_no_fee_and_forward_event(
        &self,
        context: SwapContext<Self::Api>,
        destination: ManagedAddress,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        self.swap_no_fee_and_forward_event(
            &context.output_token_id.clone(),
            &caller,
            epoch,
            &SwapNoFeeAndForwardEvent {
                caller: caller.clone(),
                token_id_in: context.input_token_id,
                token_amount_in: context.input_token_amount,
                token_id_out: context.output_token_id,
                token_amount_out: context.final_output_amount,
                destination,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_add_liquidity_event(
        &self,
        storage_cache: &StorageCache<Self>,
        context: AddLiquidityContext<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        self.add_liquidity_event(
            &storage_cache.first_token_id,
            &storage_cache.second_token_id,
            &caller,
            epoch,
            &AddLiquidityEvent {
                caller: caller.clone(),
                first_token_id: storage_cache.first_token_id.clone(),
                first_token_amount: context.first_token_optimal_amount,
                second_token_id: storage_cache.second_token_id.clone(),
                second_token_amount: context.second_token_optimal_amount,
                lp_token_id: storage_cache.lp_token_id.clone(),
                lp_token_amount: context.liq_added,
                lp_supply: storage_cache.lp_token_supply.clone(),
                first_token_reserves: storage_cache.first_token_reserve.clone(),
                second_token_reserves: storage_cache.second_token_reserve.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_remove_liquidity_event(
        &self,
        storage_cache: &StorageCache<Self>,
        context: RemoveLiquidityContext<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        self.remove_liquidity_event(
            &storage_cache.first_token_id,
            &storage_cache.second_token_id,
            &caller,
            epoch,
            &RemoveLiquidityEvent {
                caller: caller.clone(),
                first_token_id: storage_cache.first_token_id.clone(),
                first_token_amount: context.first_token_amount_removed,
                second_token_id: storage_cache.second_token_id.clone(),
                second_token_amount: context.second_token_amount_removed,
                lp_token_id: storage_cache.lp_token_id.clone(),
                lp_token_amount: context.lp_token_payment_amount,
                lp_supply: storage_cache.lp_token_supply.clone(),
                first_token_reserves: storage_cache.first_token_reserve.clone(),
                second_token_reserves: storage_cache.second_token_reserve.clone(),
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
