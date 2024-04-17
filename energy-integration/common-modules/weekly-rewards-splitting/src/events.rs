multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub use common_types::{Epoch, Week};
use energy_query::Energy;

#[multiversx_sc::module]
pub trait WeeklyRewardsSplittingEventsModule {
    #[inline]
    fn emit_claim_multi_event(
        self,
        user: &ManagedAddress,
        current_week: Week,
        energy: &Energy<Self::Api>,
        all_payments: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) {
        if all_payments.is_empty() {
            return;
        }
        self.claim_multi_event(user, current_week, energy, all_payments);
    }

    #[inline]
    fn emit_update_user_energy_event(
        self,
        user: &ManagedAddress,
        current_week: Week,
        energy: &Energy<Self::Api>,
    ) {
        self.update_user_energy_event(user, current_week, energy);
    }

    #[inline]
    fn emit_update_global_amounts_event(
        self,
        current_week: Week,
        total_locked_tokens: &BigUint,
        total_energy: &BigUint,
    ) {
        self.update_global_amounts_event(current_week, total_locked_tokens, total_energy);
    }

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
