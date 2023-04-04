multiversx_sc::imports!();

use crate::LockedFunds;
use common_structs::{Epoch, PaymentsVec};

#[multiversx_sc::module]
pub trait LkmexTransferEventsModule {
    fn emit_withdraw_event(
        self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    ) {
        self.withdraw_event(sender, receiver, payments);
    }

    fn emit_cancel_transfer_event(
        &self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        locked_funds: LockedFunds<Self::Api>,
    ) {
        self.cancel_transfer_event(sender, receiver, locked_funds);
    }

    fn emit_lock_funds_event(
        &self,
        sender: ManagedAddress,
        receiver: ManagedAddress,
        current_epoch: Epoch,
        payments: PaymentsVec<Self::Api>,
    ) {
        self.lock_funds_event(sender, receiver, current_epoch, payments);
    }

    #[event("withdraw_event")]
    fn withdraw_event(
        &self,
        #[indexed] sender: ManagedAddress,
        #[indexed] receiver: ManagedAddress,
        payments: PaymentsVec<Self::Api>,
    );

    #[event("cancel_transfer_event")]
    fn cancel_transfer_event(
        &self,
        #[indexed] sender: ManagedAddress,
        #[indexed] receiver: ManagedAddress,
        locked_funds: LockedFunds<Self::Api>,
    );

    #[event("lock_funds_event")]
    fn lock_funds_event(
        &self,
        #[indexed] sender: ManagedAddress,
        #[indexed] receiver: ManagedAddress,
        #[indexed] current_epoch: Epoch,
        payments: PaymentsVec<Self::Api>,
    );
}
