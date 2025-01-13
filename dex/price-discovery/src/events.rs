use crate::{Block, Epoch, Timestamp};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode)]
pub struct UserDepositEvent<'a, M: ManagedTypeApi> {
    token_amount_in: &'a BigUint<M>,
    redeem_token_id: &'a TokenIdentifier<M>,
    redeem_token_amount: &'a BigUint<M>,
    accepted_token_amount: &'a BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct UserWithdrawEvent<'a, M: ManagedTypeApi> {
    token_amount_out: &'a BigUint<M>,
    redeem_token_id: &'a TokenIdentifier<M>,
    redeem_token_amount: &'a BigUint<M>,
    accepted_token_amount: &'a BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct OwnerDepositEvent<'a, M: ManagedTypeApi> {
    token_amount_in: &'a BigUint<M>,
    launched_token_amount: &'a BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct OwnerWithdrawEvent<'a, M: ManagedTypeApi> {
    token_amount_out: &'a BigUint<M>,
    launched_token_amount: &'a BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct RedeemEvent<'a, M: ManagedTypeApi> {
    opt_redeem_token_id: Option<&'a TokenIdentifier<M>>,
    redeem_token_amount: &'a BigUint<M>,
    bought_token_id: &'a EgldOrEsdtTokenIdentifier<M>,
    bought_token_amount: &'a BigUint<M>,
}

pub struct GenericEventData<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    block: Block,
    epoch: Epoch,
    timestamp: Timestamp,
}

#[multiversx_sc::module]
pub trait EventsModule: crate::common_storage::CommonStorageModule {
    fn emit_user_deposit_event(
        &self,
        token_amount_in: &BigUint,
        redeem_token_id: &TokenIdentifier,
        redeem_token_amount: &BigUint,
    ) {
        let generic_event_data = self.get_generic_event_data();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.user_deposit_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            UserDepositEvent {
                token_amount_in,
                redeem_token_id,
                redeem_token_amount,
                accepted_token_amount: &accepted_token_amount,
            },
        );
    }

    fn emit_user_withdraw_event(
        &self,
        token_amount_out: &BigUint,
        redeem_token_id: &TokenIdentifier,
        redeem_token_amount: &BigUint,
    ) {
        let generic_event_data = self.get_generic_event_data();
        let accepted_token_amount = self.accepted_token_balance().get();

        self.user_withdraw_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            UserWithdrawEvent {
                token_amount_out,
                redeem_token_id,
                redeem_token_amount,
                accepted_token_amount: &accepted_token_amount,
            },
        );
    }

    fn emit_owner_deposit_event(&self, token_amount_in: &BigUint) {
        let generic_event_data = self.get_generic_event_data();
        let launched_token_amount = self.launched_token_balance().get();

        self.owner_deposit_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            OwnerDepositEvent {
                token_amount_in,
                launched_token_amount: &launched_token_amount,
            },
        );
    }

    fn emit_owner_withdraw_event(&self, token_amount_out: &BigUint) {
        let generic_event_data = self.get_generic_event_data();
        let launched_token_amount = self.launched_token_balance().get();

        self.owner_withdraw_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            OwnerWithdrawEvent {
                token_amount_out,
                launched_token_amount: &launched_token_amount,
            },
        );
    }

    fn emit_redeem_event(
        &self,
        opt_redeem_token_id: Option<&TokenIdentifier>,
        redeem_token_amount: &BigUint,
        bought_token_id: &EgldOrEsdtTokenIdentifier,
        bought_token_amount: &BigUint,
    ) {
        let generic_event_data = self.get_generic_event_data();

        self.redeem_event(
            &generic_event_data.caller,
            generic_event_data.block,
            generic_event_data.epoch,
            generic_event_data.timestamp,
            RedeemEvent {
                opt_redeem_token_id,
                redeem_token_amount,
                bought_token_id,
                bought_token_amount,
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

    #[event("userDepositEvent")]
    fn user_deposit_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        deposit_event: UserDepositEvent<Self::Api>,
    );

    #[event("userWithdrawEvent")]
    fn user_withdraw_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        withdraw_event: UserWithdrawEvent<Self::Api>,
    );

    #[event("ownerDepositEvent")]
    fn owner_deposit_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        deposit_event: OwnerDepositEvent<Self::Api>,
    );

    #[event("ownerWithdrawEvent")]
    fn owner_withdraw_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: Block,
        #[indexed] epoch: Epoch,
        #[indexed] timestamp: Timestamp,
        withdraw_event: OwnerWithdrawEvent<Self::Api>,
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
