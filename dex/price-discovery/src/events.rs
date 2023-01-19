use crate::phase::Phase;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode)]
pub struct DepositEvent<M: ManagedTypeApi> {
    token_id_in: EgldOrEsdtTokenIdentifier<M>,
    token_amount_in: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct WithdrawEvent<M: ManagedTypeApi> {
    token_id_out: EgldOrEsdtTokenIdentifier<M>,
    token_amount_out: BigUint<M>,
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    launched_token_amount: BigUint<M>,
    accepted_token_amount: BigUint<M>,
    current_price: BigUint<M>,
    current_phase: Phase<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct RedeemEvent<M: ManagedTypeApi> {
    redeem_token_id: TokenIdentifier<M>,
    redeem_token_nonce: u64,
    redeem_token_amount: BigUint<M>,
    bought_token_id: EgldOrEsdtTokenIdentifier<M>,
    bought_token_amount: BigUint<M>,
}

#[multiversx_sc::module]
pub trait EventsModule: crate::common_storage::CommonStorageModule {
    fn emit_deposit_event(
        &self,
        token_id_in: EgldOrEsdtTokenIdentifier,
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
        token_id_out: EgldOrEsdtTokenIdentifier,
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
        bought_token_id: EgldOrEsdtTokenIdentifier,
        bought_token_amount: BigUint,
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
                bought_token_id,
                bought_token_amount,
            },
        )
    }

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
