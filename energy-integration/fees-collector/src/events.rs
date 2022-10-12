elrond_wasm::imports!();

use common_types::Week;

#[elrond_wasm::module]
pub trait FeesCollectorEventsModule {
    fn emit_deposit_swap_fees_event(
        self,
        caller: ManagedAddress,
        current_week: Week,
        payment_token: TokenIdentifier,
        payment_amount: BigUint,
    ) {
        self.deposit_swap_fees_event(caller, current_week, payment_token, payment_amount);
    }

    #[event("deposit_swap_fees_event")]
    fn deposit_swap_fees_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] current_week: Week,
        #[indexed] payment_token: TokenIdentifier,
        payment_amount: BigUint,
    );
}
