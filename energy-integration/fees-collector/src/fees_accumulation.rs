elrond_wasm::imports!();

use crate::week_timekeeping::Week;

#[elrond_wasm::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule + crate::week_timekeeping::WeekTimekeepingModule
{
    #[payable("*")]
    #[endpoint(depositSwapFees)]
    fn deposit_swap_fees(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            self.known_pair_contracts().contains(&caller),
            "Only pair contracts can deposit"
        );

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        require!(
            self.known_tokens().contains(&payment_token),
            "Invalid payment token"
        );

        let current_week = self.get_current_week();
        self.accumulated_fees(current_week, &payment_token)
            .update(|amt| *amt += payment_amount);
    }

    #[view(getAccumulatedFeesForWeek)]
    fn get_accumulated_fees_for_week(
        &self,
        week: Week,
    ) -> MultiValueEncoded<MultiValue2<TokenIdentifier, BigUint>> {
        let mut results = MultiValueEncoded::new();
        let all_tokens = self.all_tokens().get();
        for token in &all_tokens {
            let accumulated_fees = self.accumulated_fees(week, &token).get();
            results.push((token, accumulated_fees).into());
        }

        results
    }

    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;
}
