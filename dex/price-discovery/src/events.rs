use crate::{phase::Phase, Block, Epoch, Nonce, Timestamp};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode)]
pub struct DepositEvent<M: ManagedTypeApi> {
    token_id_in: EgldOrEsdtTokenIdentifier<M>,
    token_amount_in: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: Nonce,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase,
}

pub struct DepositEventArgs<M: ManagedTypeApi> {
    pub token_id_in: EgldOrEsdtTokenIdentifier<M>,
    pub token_amount_in: BigUint<M>,
    pub redeem_token_id: TokenIdentifier<M>,
    pub redeem_token_nonce: Nonce,
    pub redeem_token_amount: BigUint<M>,
    pub current_price: BigUint<M>,
    pub current_phase: Phase,
}

#[derive(TypeAbi, TopEncode)]
pub struct WithdrawEvent<M: ManagedTypeApi> {
    token_id_out: EgldOrEsdtTokenIdentifier<M>,
    token_amount_out: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: Nonce,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase,
}

pub struct WithdrawEventArgs<M: ManagedTypeApi> {
    pub token_id_out: EgldOrEsdtTokenIdentifier<M>,
    pub token_amount_out: BigUint<M>,
    pub redeem_token_id: TokenIdentifier<M>,
    pub redeem_token_nonce: Nonce,
    pub redeem_token_amount: BigUint<M>,
    pub current_price: BigUint<M>,
    pub current_phase: Phase,
}

#[derive(TypeAbi, TopEncode)]
pub struct RedeemEvent<M: ManagedTypeApi> {
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: Nonce,
    redeem_token_amount: BigUint<M>,
    bought_token_id: EgldOrEsdtTokenIdentifier<M>,
    bought_token_amount: BigUint<M>,
}

pub struct RedeemEventArgs<M: ManagedTypeApi> {
    pub redeem_token_id: TokenIdentifier<M>,
    pub redeem_token_nonce: Nonce,
    pub redeem_token_amount: BigUint<M>,
    pub bought_token_id: EgldOrEsdtTokenIdentifier<M>,
    pub bought_token_amount: BigUint<M>,
}

pub struct GenericEventData<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    block: Block,
    epoch: Epoch,
    timestamp: Timestamp,
}

#[multiversx_sc::module]
pub trait EventsModule: crate::common_storage::CommonStorageModule {
    fn emit_deposit_event(&self, args: DepositEventArgs<Self::Api>) {
        let generic_event_data = self.get_generic_event_data();
        let launched_token_amount = self.launched_token_balance().get();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.deposit_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            DepositEvent {
                token_id_in: args.token_id_in,
                token_amount_in: args.token_amount_in,
                redeem_token_id: args.redeem_token_id,
                redeem_token_nonce: args.redeem_token_nonce,
                redeem_token_amount: args.redeem_token_amount,
                launched_token_amount,
                accepted_token_amount,
                current_price: args.current_price,
                current_phase: args.current_phase,
            },
        );
    }

    fn emit_withdraw_event(&self, args: WithdrawEventArgs<Self::Api>) {
        let generic_event_data = self.get_generic_event_data();
        let launched_token_amount = self.launched_token_balance().get();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.withdraw_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            WithdrawEvent {
                token_id_out: args.token_id_out,
                token_amount_out: args.token_amount_out,
                redeem_token_id: args.redeem_token_id,
                redeem_token_nonce: args.redeem_token_nonce,
                redeem_token_amount: args.redeem_token_amount,
                launched_token_amount,
                accepted_token_amount,
                current_price: args.current_price,
                current_phase: args.current_phase,
            },
        );
    }

    fn emit_redeem_event(&self, args: RedeemEventArgs<Self::Api>) {
        let generic_event_data = self.get_generic_event_data();

        self.redeem_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            RedeemEvent {
                redeem_token_id: args.redeem_token_id,
                redeem_token_nonce: args.redeem_token_nonce,
                redeem_token_amount: args.redeem_token_amount,
                bought_token_id: args.bought_token_id,
                bought_token_amount: args.bought_token_amount,
            },
        )
    }

    fn get_generic_event_data(&self) -> GenericEventData<Self::Api> {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();

        GenericEventData {
            caller,
            block,
            epoch,
            timestamp,
        }
    }

    #[event("depositEvent")]
    fn deposit_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        deposit_event: DepositEvent<Self::Api>,
    );

    #[event("withdrawEvent")]
    fn withdraw_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        withdraw_event: WithdrawEvent<Self::Api>,
    );

    #[event("redeemEvent")]
    fn redeem_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        redeem_event: RedeemEvent<Self::Api>,
    );
}
