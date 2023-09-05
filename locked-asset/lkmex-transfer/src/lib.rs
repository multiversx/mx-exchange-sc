#![no_std]
#![allow(clippy::vec_init_then_push)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod constants;
pub mod energy_transfer;
pub mod events;

use common_structs::{Epoch, PaymentsVec};
use permissions_module::Permissions;

use crate::constants::*;

#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, ManagedVecItem, TypeAbi, Debug,
)]
pub struct LockedFunds<M: ManagedTypeApi> {
    pub funds: PaymentsVec<M>,
    pub locked_epoch: Epoch,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct ScheduledTransfer<M: ManagedTypeApi> {
    pub sender: ManagedAddress<M>,
    pub locked_funds: LockedFunds<M>,
}

#[multiversx_sc::contract]
pub trait LkmexTransfer:
    energy_transfer::EnergyTransferModule
    + events::LkmexTransferEventsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
    + permissions_module::PermissionsModule
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

        let caller = self.blockchain().get_caller();
        self.add_permissions(caller, Permissions::OWNER);
    }

    #[endpoint]
    fn withdraw(&self, sender: ManagedAddress) {
        let receiver = self.blockchain().get_caller();
        let receiver_last_transfer_mapper = self.receiver_last_transfer_epoch(&receiver);
        self.check_address_on_cooldown(&receiver_last_transfer_mapper);
        let funds = self.get_unlocked_funds(&receiver, &sender);
        self.add_energy_to_destination(receiver.clone(), &funds);
        self.send().direct_multi(&receiver, &funds);
        self.locked_funds(&receiver, &sender).clear();
        self.all_senders(&receiver).swap_remove(&sender);

        let current_epoch = self.blockchain().get_block_epoch();
        receiver_last_transfer_mapper.set(current_epoch);

        let locked_funds = LockedFunds {
            funds,
            locked_epoch: current_epoch,
        };
        self.emit_withdraw_event(sender, receiver, locked_funds);
    }

    #[endpoint(cancelTransfer)]
    fn cancel_transfer(&self, sender: ManagedAddress, receiver: ManagedAddress) {
        self.require_caller_has_admin_permissions();
        let locked_funds_mapper = self.locked_funds(&receiver, &sender);
        require!(!locked_funds_mapper.is_empty(), TRANSFER_NON_EXISTENT);

        let locked_funds = locked_funds_mapper.get();
        locked_funds_mapper.clear();
        self.all_senders(&receiver).swap_remove(&sender);
        self.sender_last_transfer_epoch(&sender).clear();

        self.add_energy_to_destination(sender.clone(), &locked_funds.funds);
        self.send().direct_multi(&sender, &locked_funds.funds);

        self.emit_cancel_transfer_event(sender, receiver, locked_funds);
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
        let sender_last_transfer_mapper = self.sender_last_transfer_epoch(&sender);
        self.check_address_on_cooldown(&sender_last_transfer_mapper);

        let payments = self.call_value().all_esdt_transfers().clone_value();
        let locked_token_id = self.locked_token_id().get();
        for payment in payments.iter() {
            require!(
                payment.token_identifier == locked_token_id,
                BAD_LOCKING_TOKEN
            )
        }

        self.deduct_energy_from_sender(sender.clone(), &payments);

        let current_epoch = self.blockchain().get_block_epoch();
        let locked_funds = LockedFunds {
            funds: payments,
            locked_epoch: current_epoch,
        };
        self.locked_funds(&receiver, &sender)
            .set(locked_funds.clone());
        sender_last_transfer_mapper.set(current_epoch);
        self.all_senders(&receiver).insert(sender.clone());

        self.emit_lock_funds_event(sender, receiver, locked_funds);
    }

    fn check_address_on_cooldown(&self, last_transfer_mapper: &SingleValueMapper<Epoch>) {
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

    #[view(getScheduledTransfers)]
    fn get_scheduled_transfers(
        &self,
        receiver: ManagedAddress,
    ) -> MultiValueEncoded<ScheduledTransfer<Self::Api>> {
        let mut result = MultiValueEncoded::new();
        for sender in self.all_senders(&receiver).iter() {
            let locked_funds = self.locked_funds(&receiver, &sender).get();
            let scheduled_transfer = ScheduledTransfer {
                sender,
                locked_funds,
            };

            result.push(scheduled_transfer);
        }

        result
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

    #[storage_mapper("senderLastTransferEpoch")]
    fn sender_last_transfer_epoch(&self, sender: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("receiverLastTransferEpoch")]
    fn receiver_last_transfer_epoch(&self, receiver: &ManagedAddress) -> SingleValueMapper<Epoch>;

    #[storage_mapper("lockedTokenId")]
    fn locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("minLockEpochs")]
    fn min_lock_epochs(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("epochsCooldownDuration")]
    fn epochs_cooldown_duration(&self) -> SingleValueMapper<Epoch>;
}
