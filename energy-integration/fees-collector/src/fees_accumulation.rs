elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use week_timekeeping::Week;

static BASE_TOKEN_ID_STORAGE_KEY: &[u8] = b"baseAssetTokenId";

#[elrond_wasm::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + utils::UtilsModule
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
        let locked_token_id = self.locked_token_id().get();
        if payment.token_identifier == locked_token_id {
            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        } else {
            require!(
                self.known_tokens().contains(&payment.token_identifier),
                "Invalid payment token"
            );
        }

        let current_week = self.get_current_week();
        self.accumulated_fees(current_week, &payment.token_identifier)
            .update(|amt| *amt += &payment.amount);

        self.emit_deposit_swap_fees_event(caller, current_week, payment);
    }

    fn get_and_clear_acccumulated_fees(&self, week: Week, token: &TokenIdentifier) -> BigUint {
        let mapper = self.accumulated_fees(week, token);
        let value = mapper.get();
        if value > 0 {
            mapper.clear();
        }

        value
    }

    fn get_base_token_id(&self, energy_factory_addr: &ManagedAddress) -> TokenIdentifier {
        self.storage_raw().read_from_address(
            energy_factory_addr,
            ManagedBuffer::new_from_bytes(BASE_TOKEN_ID_STORAGE_KEY),
        )
    }

    #[view(getAccumulatedFees)]
    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;
}
