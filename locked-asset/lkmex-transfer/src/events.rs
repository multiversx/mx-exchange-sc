multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::LockedFunds;

#[derive(TypeAbi, TopEncode)]
pub struct LkmexTransferEvent<M: ManagedTypeApi> {
    sender: ManagedAddress<M>,
    receiver: ManagedAddress<M>,
    locked_funds: LockedFunds<M>,
}

#[multiversx_sc::module]
pub trait LkmexTransferEventsModule {
    fn emit_withdraw_event(
        self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        locked_funds: LockedFunds<Self::Api>,
    ) {
        let event_data = LkmexTransferEvent {
            sender,
            receiver,
            locked_funds,
        };
        self.withdraw_event(event_data);
    }

    fn emit_cancel_transfer_event(
        &self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        locked_funds: LockedFunds<Self::Api>,
    ) {
        let event_data = LkmexTransferEvent {
            sender,
            receiver,
            locked_funds,
        };
        self.cancel_transfer_event(event_data);
    }

    fn emit_lock_funds_event(
        &self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        locked_funds: LockedFunds<Self::Api>,
    ) {
        let event_data = LkmexTransferEvent {
            sender,
            receiver,
            locked_funds,
        };
        self.lock_funds_event(event_data);
    }

    #[event("withdraw_event")]
    fn withdraw_event(&self, event_data: LkmexTransferEvent<Self::Api>);

    #[event("cancel_transfer_event")]
    fn cancel_transfer_event(&self, event_data: LkmexTransferEvent<Self::Api>);

    #[event("lock_funds_event")]
    fn lock_funds_event(&self, event_data: LkmexTransferEvent<Self::Api>);
}
