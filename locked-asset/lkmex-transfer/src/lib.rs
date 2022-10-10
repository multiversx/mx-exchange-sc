#![no_std]
#![allow(clippy::vec_init_then_push)]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod constants;
use crate::constants::*;

type Epoch = u64;

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct LockedFunds<M: ManagedTypeApi> {
    funds: ManagedVec<M, EsdtTokenPayment<M>>,
    time: Epoch,
}

#[elrond_wasm::contract]
pub trait LkmexTransfer {
    #[init]
    fn init(
        &self,
        token: TokenIdentifier,
        unlock_transfer_time: Epoch,
        epochs_cooldown_duration: Epoch,
    ) {
        self.unlock_transfer_time().set(unlock_transfer_time);
        self.epochs_cooldown_duration()
            .set(epochs_cooldown_duration);
        self.locked_token().set(token);
    }

    #[endpoint(withdraw)]
    fn withdraw(&self) {
        let caller = self.blockchain().get_caller();
        let funds = self.get_unlocked_funds(&caller);
        self.send().direct_multi(&caller, &funds);
        self.locked_funds(&caller).clear();
    }

    fn get_unlocked_funds(&self, address: &ManagedAddress) -> ManagedVec<EsdtTokenPayment> {
        require!(
            !self.locked_funds(address).is_empty(),
            CALLER_NOTHING_TO_CLAIM
        );

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_transfer_time = self.unlock_transfer_time().get();
        let locked_funds = self.locked_funds(address).get();
        require!(
            current_epoch - locked_funds.time > unlock_transfer_time,
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
        for payment in payments.iter() {
            require!(
                payment.token_identifier == self.locked_token().get(),
                BAD_LOCKING_TOKEN
            )
        }

        let current_epoch = self.blockchain().get_block_epoch();
        self.locked_funds(&address).set(LockedFunds {
            funds: payments,
            time: current_epoch,
        });
        self.address_last_transfer_epoch(&caller).set(current_epoch);
    }

    fn check_address_on_cooldown(&self, address: &ManagedAddress) {
        if !self.address_last_transfer_epoch(address).is_empty() {
            let current_epoch = self.blockchain().get_block_epoch();
            let epochs_cooldown_duration = self.epochs_cooldown_duration().get();
            let last_transfer_epoch = self.address_last_transfer_epoch(address).get();
            let epochs_since_last_transfer = current_epoch - last_transfer_epoch;
            require!(
                epochs_since_last_transfer > epochs_cooldown_duration,
                CALLER_ON_COOLDOWN
            )
        }
    }

    #[storage_mapper("locked_funds")]
    fn locked_funds(&self, owner: &ManagedAddress) -> SingleValueMapper<LockedFunds<Self::Api>>;

    #[storage_mapper("address_last_transfer_epoch")]
    fn address_last_transfer_epoch(&self, owner: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("locked_token")]
    fn locked_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("unlock_transfer_time")]
    fn unlock_transfer_time(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("epochs_cooldown_duration")]
    fn epochs_cooldown_duration(&self) -> SingleValueMapper<Epoch>;
}
