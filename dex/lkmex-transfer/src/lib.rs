#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod constants;
use crate::constants::*;

#[elrond_wasm::contract]
pub trait LkmexTransfer {
    #[init]
    fn init(&self, unlock_transfer_time: u64, address_cooldown_duration: u64) {
        self.unlock_transfer_time().set(unlock_transfer_time);
        self.address_cooldown_duration()
            .set(address_cooldown_duration);
    }

    #[endpoint(transfer)]
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
        let (funds, lock_time) = self.locked_funds(address).get();
        require!(
            current_epoch - lock_time > unlock_transfer_time,
            TOKENS_STILL_LOCKED
        );
        funds
    }

    #[payable("*")]
    #[endpoint(lockFunds)]
    fn lock_funds(&self, address: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        self.check_address_on_cooldown(&caller);

        let payments = self.call_value().all_esdt_transfers();
        for payment in payments.iter() {
            require!(
                payment.token_identifier == TokenIdentifier::from(LPMEX_TOKEN_ID),
                BAD_LOCKING_TOKEN
            )
        }

        let current_epoch = self.blockchain().get_block_epoch();
        self.locked_funds(&address).set((payments, current_epoch));
        self.address_last_transfer_epoch(&caller).set(current_epoch);
    }

    fn check_address_on_cooldown(&self, address: &ManagedAddress) {
        if !self.address_last_transfer_epoch(address).is_empty() {
            let current_epoch = self.blockchain().get_block_epoch();
            let address_cooldown_duration = self.address_cooldown_duration().get();
            let last_cooldown = self.address_last_transfer_epoch(address).get();
            require!(
                current_epoch - last_cooldown > address_cooldown_duration,
                CALLER_ON_COOLDOWN
            )
        }
    }

    #[storage_mapper("locked_funds")]
    fn locked_funds(
        &self,
        owner: &ManagedAddress,
    ) -> SingleValueMapper<(ManagedVec<EsdtTokenPayment<Self::Api>>, u64)>;

    #[storage_mapper("address_last_transfer_epoch")]
    fn address_last_transfer_epoch(&self, owner: &ManagedAddress) -> SingleValueMapper<u64>;

    #[storage_mapper("unlock_transfer_time")]
    fn unlock_transfer_time(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("address_cooldown_duration")]
    fn address_cooldown_duration(&self) -> SingleValueMapper<u64>;
}
