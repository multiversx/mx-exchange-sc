elrond_wasm::imports!();

use crate::locked_asset_token::{PaymentsVec, UserEntry};

#[elrond_wasm::module]
pub trait EventsModule {
    #[event("stakeEvent")]
    fn stake_event(
        &self,
        #[indexed] user_address: &ManagedAddress,
        #[indexed] tokens: &PaymentsVec<Self::Api>,
        entry_after_action: &UserEntry<Self::Api>,
    );

    #[event("unstakeEvent")]
    fn unstake_event(
        &self,
        #[indexed] user_address: &ManagedAddress,
        #[indexed] amount: &BigUint,
        #[indexed] unbond_epoch: u64,
        entry_after_action: &UserEntry<Self::Api>,
    );

    #[event("unbondEvent")]
    fn unbond_event(
        &self,
        #[indexed] user_address: &ManagedAddress,
        #[indexed] amount: &BigUint,
        opt_entry_after_action: Option<&UserEntry<Self::Api>>,
    );
}
