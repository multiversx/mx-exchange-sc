use common_structs::{Epoch, Percent};

use crate::Blocks;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode)]
pub struct CreatePairEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    first_token_id: TokenIdentifier<M>,
    second_token_id: TokenIdentifier<M>,
    total_fee_percent: Percent,
    special_fee_percent: Percent,
    pair_address: ManagedAddress<M>,
    block: Blocks,
    epoch: Epoch,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct UserPairSwapEnabledEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    first_token_id: TokenIdentifier<M>,
    second_token_id: TokenIdentifier<M>,
    pair_address: ManagedAddress<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct MultiPairSwapEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    token_in: TokenIdentifier<M>,
    amount_in: BigUint<M>,
    token_out: TokenIdentifier<M>,
    amount_out: BigUint<M>,
    payments_out: ManagedVec<M, EsdtTokenPayment<M>>,
    block: Blocks,
    epoch: Epoch,
    timestamp: u64,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_create_pair_event(
        self,
        caller: ManagedAddress,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        total_fee_percent: Percent,
        special_fee_percent: Percent,
        pair_address: ManagedAddress,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.create_pair_event(
            first_token_id.clone(),
            second_token_id.clone(),
            caller.clone(),
            epoch,
            CreatePairEvent {
                caller,
                first_token_id,
                second_token_id,
                total_fee_percent,
                special_fee_percent,
                pair_address,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_user_swaps_enabled_event(
        &self,
        caller: ManagedAddress,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        pair_address: ManagedAddress,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.pair_swap_enabled_event(
            first_token_id.clone(),
            second_token_id.clone(),
            caller.clone(),
            epoch,
            UserPairSwapEnabledEvent {
                caller,
                first_token_id,
                second_token_id,
                pair_address,
            },
        )
    }

    fn emit_multi_pair_swap_event(
        &self,
        caller: ManagedAddress,
        token_in: TokenIdentifier,
        amount_in: BigUint,
        payments_out: ManagedVec<EsdtTokenPayment>,
    ) {
        if payments_out.is_empty() {
            return;
        }

        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        let last_payment_index = payments_out.len() - 1;
        let token_out = payments_out.get(last_payment_index);
        self.multi_pair_swap_event(
            caller.clone(),
            token_in.clone(),
            amount_in.clone(),
            token_out.token_identifier.clone(),
            token_out.amount.clone(),
            epoch,
            MultiPairSwapEvent {
                caller,
                token_in,
                amount_in,
                token_out: token_out.token_identifier,
                amount_out: token_out.amount,
                payments_out,
                block,
                epoch,
                timestamp,
            },
        )
    }

    #[event("create_pair")]
    fn create_pair_event(
        self,
        #[indexed] first_token_id: TokenIdentifier,
        #[indexed] second_token_id: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] epoch: Epoch,
        swap_event: CreatePairEvent<Self::Api>,
    );

    #[event("pairSwapEnabled")]
    fn pair_swap_enabled_event(
        &self,
        #[indexed] first_token_id: TokenIdentifier,
        #[indexed] second_token_id: TokenIdentifier,
        #[indexed] caller: ManagedAddress,
        #[indexed] epoch: Epoch,
        swap_enabled_event: UserPairSwapEnabledEvent<Self::Api>,
    );

    #[event("multiPairSwap")]
    fn multi_pair_swap_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] token_in: TokenIdentifier,
        #[indexed] amount_in: BigUint,
        #[indexed] token_out: TokenIdentifier,
        #[indexed] amount_out: BigUint,
        #[indexed] epoch: Epoch,
        multi_pair_swap_event: MultiPairSwapEvent<Self::Api>,
    );
}
