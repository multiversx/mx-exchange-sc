#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod constants;

use common_structs::{Epoch, PaymentsVec};

use crate::constants::*;

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct LockedFunds<M: ManagedTypeApi> {
    funds: PaymentsVec<M>,
    unlock_epoch: Epoch,
}

#[elrond_wasm::contract]
pub trait LkmexTransfer {
    #[init]
    fn init(
        &self,
        locked_token_id: TokenIdentifier,
        min_lock_epochs: Epoch,
        epochs_cooldown_duration: Epoch,
    ) {
        self.min_lock_epochs().set(min_lock_epochs);
        self.epochs_cooldown_duration()
            .set(epochs_cooldown_duration);
        self.locked_token_id().set(locked_token_id);
    }

    #[endpoint(withdraw)]
    fn withdraw(&self) {
        let caller = self.blockchain().get_caller();
        let funds = self.get_unlocked_funds(&caller);
        self.send().direct_multi(&caller, &funds);
        self.locked_funds(&caller).clear();

        let current_epoch = self.blockchain().get_block_epoch();
        self.address_last_transfer_epoch(&caller).set(current_epoch);
    }

    fn get_unlocked_funds(&self, address: &ManagedAddress) -> PaymentsVec<Self::Api> {
        require!(
            !self.locked_funds(address).is_empty(),
            CALLER_NOTHING_TO_CLAIM
        );

        let current_epoch = self.blockchain().get_block_epoch();
        let min_lock_epochs = self.min_lock_epochs().get();
        let locked_funds = self.locked_funds(address).get();
        require!(
            current_epoch - locked_funds.unlock_epoch > min_lock_epochs,
            TOKENS_STILL_LOCKED
        );

        locked_funds.funds
    }

    #[payable("*")]
    #[endpoint(lockFunds)]
    fn lock_funds(&self, address: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        self.check_address_on_cooldown(&caller);

        let payments = self.call_value().all_esdt_transfers();
        let locked_token_id = self.locked_token_id().get();
        for payment in payments.iter() {
            require!(
                payment.token_identifier == locked_token_id,
                BAD_LOCKING_TOKEN
            )
        }

        let current_epoch = self.blockchain().get_block_epoch();
        self.locked_funds(&address).set(LockedFunds {
            funds: payments,
            unlock_epoch: current_epoch,
        });
        self.address_last_transfer_epoch(&caller).set(current_epoch);
    }

    fn check_address_on_cooldown(&self, address: &ManagedAddress) {
        let last_transfer_mapper = self.address_last_transfer_epoch(address);
        if last_transfer_mapper.is_empty() {
            return;
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let epochs_cooldown_duration = self.epochs_cooldown_duration().get();
        let last_transfer_epoch = last_transfer_mapper.get();
        let epochs_since_last_transfer = current_epoch - last_transfer_epoch;
        require!(
            epochs_since_last_transfer > epochs_cooldown_duration,
            CALLER_ON_COOLDOWN
        )
    }

    #[storage_mapper("lockedFunds")]
    fn locked_funds(&self, owner: &ManagedAddress) -> SingleValueMapper<LockedFunds<Self::Api>>;

    #[storage_mapper("addressLastTransferEpoch")]
    fn address_last_transfer_epoch(&self, owner: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("lockedTokenId")]
    fn locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("minLockEpochs")]
    fn min_lock_epochs(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("epochsCooldownDuration")]
    fn epochs_cooldown_duration(&self) -> SingleValueMapper<Epoch>;
}
