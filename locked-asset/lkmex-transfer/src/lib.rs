#![no_std]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod constants;
pub mod energy_transfer;

use common_structs::{Epoch, PaymentsVec};

use crate::constants::*;

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct LockedFunds<M: ManagedTypeApi> {
    funds: PaymentsVec<M>,
    locked_epoch: Epoch,
}

#[elrond_wasm::contract]
pub trait LkmexTransfer:
    energy_transfer::EnergyTransferModule + energy_query::EnergyQueryModule + utils::UtilsModule
{
    #[init]
    fn init(
        &self,
        energy_factory_address: ManagedAddress,
        locked_token_id: TokenIdentifier,
        min_lock_epochs: Epoch,
        epochs_cooldown_duration: Epoch,
    ) {
        self.require_valid_token_id(&locked_token_id);

        self.min_lock_epochs().set(min_lock_epochs);
        self.epochs_cooldown_duration()
            .set(epochs_cooldown_duration);
        self.locked_token_id().set(locked_token_id);
        self.set_energy_factory_address(energy_factory_address);
    }

    #[endpoint]
    fn withdraw(&self, sender: ManagedAddress) {
        let receiver = self.blockchain().get_caller();
        let funds = self.get_unlocked_funds(&receiver, &sender);
        self.send().direct_multi(&receiver, &funds);
        self.locked_funds(&receiver, &sender).clear();
        self.all_senders(&receiver).swap_remove(&sender);

        let current_epoch = self.blockchain().get_block_epoch();
        self.address_last_transfer_epoch(&receiver)
            .set(current_epoch);

        self.add_energy_to_destination(receiver, &funds);
    }

    fn get_unlocked_funds(
        &self,
        receiver: &ManagedAddress,
        sender: &ManagedAddress,
    ) -> PaymentsVec<Self::Api> {
        let locked_funds_mapper = self.locked_funds(receiver, sender);
        require!(!locked_funds_mapper.is_empty(), CALLER_NOTHING_TO_CLAIM);

        let current_epoch = self.blockchain().get_block_epoch();
        let min_lock_epochs = self.min_lock_epochs().get();
        let locked_funds = locked_funds_mapper.get();
        require!(
            current_epoch - locked_funds.locked_epoch > min_lock_epochs,
            TOKENS_STILL_LOCKED
        );

        locked_funds.funds
    }

    #[payable("*")]
    #[endpoint(lockFunds)]
    fn lock_funds(&self, receiver: ManagedAddress) {
        let sender = self.blockchain().get_caller();
        let locked_funds_mapper = self.locked_funds(&receiver, &sender);
        require!(locked_funds_mapper.is_empty(), ALREADY_SENT_TO_ADDRESS);
        self.check_address_on_cooldown(&sender);

        let payments = self.call_value().all_esdt_transfers();
        let locked_token_id = self.locked_token_id().get();
        for payment in payments.iter() {
            require!(
                payment.token_identifier == locked_token_id,
                BAD_LOCKING_TOKEN
            )
        }

        self.deduct_energy_from_sender(sender.clone(), &payments);

        let current_epoch = self.blockchain().get_block_epoch();
        self.locked_funds(&receiver, &sender).set(LockedFunds {
            funds: payments,
            locked_epoch: current_epoch,
        });
        self.address_last_transfer_epoch(&sender).set(current_epoch);
        self.all_senders(&receiver).insert(sender);
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
    fn locked_funds(
        &self,
        receiver: &ManagedAddress,
        sender: &ManagedAddress,
    ) -> SingleValueMapper<LockedFunds<Self::Api>>;

    #[view(getAllSenders)]
    #[storage_mapper("allSenders")]
    fn all_senders(&self, receiver: &ManagedAddress) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("addressLastTransferEpoch")]
    fn address_last_transfer_epoch(&self, owner: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("lockedTokenId")]
    fn locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("minLockEpochs")]
    fn min_lock_epochs(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("epochsCooldownDuration")]
    fn epochs_cooldown_duration(&self) -> SingleValueMapper<Epoch>;
}
