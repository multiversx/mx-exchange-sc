#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub use common_types::{Epoch, Week};
use simple_lock_energy::energy::Energy;

#[elrond_wasm::module]
pub trait CommonEventsModule {
    fn emit_deposit_swap_fees_event(
        self,
        caller: ManagedAddress,
        current_week: Week,
        payment_token: TokenIdentifier,
        payment_amount: BigUint,
    ) {
        self.deposit_swap_fees_event(caller, current_week, payment_token, payment_amount);
    }

    fn emit_claim_multi_event(
        self,
        user: &ManagedAddress,
        current_week: Week,
        energy: &Energy<Self::Api>,
        all_payments: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) {
        self.claim_multi_event(user, current_week, energy, all_payments);
    }

    fn emit_update_user_energy_event(
        self,
        user: &ManagedAddress,
        current_week: Week,
        energy: &Energy<Self::Api>,
    ) {
        self.update_user_energy_event(user, current_week, energy);
    }

    fn emit_update_global_amounts_event(
        self,
        current_week: Week,
        total_locked_tokens: &BigUint,
        total_energy: &BigUint,
    ) {
        self.update_global_amounts_event(current_week, total_locked_tokens, total_energy);
    }

    #[event("deposit_swap_fees_event")]
    fn deposit_swap_fees_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] current_week: Week,
        #[indexed] payment_token: TokenIdentifier,
        payment_amount: BigUint,
    );

    #[event("claim_multi_event")]
    fn claim_multi_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] current_week: Week,
        #[indexed] energy: &Energy<Self::Api>,
        all_payments: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
    );

    #[event("update_user_energy_event")]
    fn update_user_energy_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] current_week: Week,
        #[indexed] energy: &Energy<Self::Api>,
    );

    #[event("update_global_amounts_event")]
    fn update_global_amounts_event(
        &self,
        #[indexed] current_week: Week,
        #[indexed] total_locked_tokens: &BigUint,
        #[indexed] total_energy: &BigUint,
    );
}
