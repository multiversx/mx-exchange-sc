multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use week_timekeeping::Week;

#[multiversx_sc::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
{
    /// Pair SC will deposit the fees through this endpoint
    /// Deposits for current week are accessible starting next week
    #[payable("*")]
    #[endpoint(depositSwapFees)]
    fn deposit_swap_fees(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            self.known_contracts().contains(&caller),
            "Only known contracts can deposit"
        );

        let payment = self.call_value().single_esdt();
        require!(
            self.known_tokens().contains(&payment.token_identifier),
            "Invalid payment token"
        );
        let current_week = self.get_current_week();

        if payment.token_nonce > 0 {
            require!(
                payment.token_identifier == self.locked_token_id().get(),
                "Invalid locked token"
            );
            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }
        self.accumulated_fees(current_week, &payment.token_identifier)
            .update(|amt| *amt += &payment.amount);

        self.emit_deposit_swap_fees_event(caller, current_week, payment);
    }

    fn get_and_clear_accumulated_fees(
        &self,
        week: Week,
        token: &TokenIdentifier,
    ) -> Option<BigUint> {
        let value = self.accumulated_fees(week, token).take();
        if value > 0 {
            Some(value)
        } else {
            None
        }
    }

    #[view(getAccumulatedFees)]
    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;
}
