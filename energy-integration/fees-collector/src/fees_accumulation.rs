elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::week_timekeeping::Week;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct TokenAmountPair<M: ManagedTypeApi> {
    pub token: TokenIdentifier<M>,
    pub amount: BigUint<M>,
}

impl<M: ManagedTypeApi> TokenAmountPair<M> {
    #[inline]
    pub fn new(token: TokenIdentifier<M>, amount: BigUint<M>) -> Self {
        TokenAmountPair { token, amount }
    }
}

#[elrond_wasm::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule + crate::week_timekeeping::WeekTimekeepingModule
{
    /// Pair SC will deposit the fees through this endpoint
    /// Deposits for current week are stored to be accessible starting next week
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

        let next_week = self.get_current_week() + 1;
        self.accumulated_fees(next_week, &payment_token)
            .update(|amt| *amt += payment_amount);
    }

    fn collect_accumulated_fees_for_week(
        &self,
        week: Week,
    ) -> ManagedVec<TokenAmountPair<Self::Api>> {
        let mut results = ManagedVec::new();
        let all_tokens = self.all_tokens().get();
        for token in &all_tokens {
            let accumulated_fees = self.get_and_clear_acccumulated_fees(week, &token);
            if accumulated_fees > 0 {
                results.push(TokenAmountPair::new(token, accumulated_fees));
            }
        }

        results
    }

    fn get_and_clear_acccumulated_fees(&self, week: Week, token: &TokenIdentifier) -> BigUint {
        let mapper = self.accumulated_fees(week, token);
        let value = mapper.get();
        mapper.clear();

        value
    }

    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;
}
