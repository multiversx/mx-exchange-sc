multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Percent;
use energy_factory::lock_options::MAX_PENALTY_PERCENTAGE;
use week_timekeeping::Week;

#[multiversx_sc::module]
pub trait FeesAccumulationModule:
    crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    /// Base token burn percent is between 0 (0%) and 10_000 (100%)
    #[only_owner]
    #[endpoint(setBaseTokenBurnPercent)]
    fn set_base_token_burn_percent(&self, burn_percent: Percent) {
        require!(burn_percent <= MAX_PENALTY_PERCENTAGE, "Invalid percent");

        self.base_token_burn_percent().set(burn_percent);
    }

    /// Anyone can deposit tokens through this endpoint
    ///
    /// Deposits for current week are accessible starting next week
    #[payable("*")]
    #[endpoint(depositSwapFees)]
    fn deposit_swap_fees(&self) {
        let mut payment = self.call_value().single_esdt();
        self.add_known_token(payment.token_identifier.clone());

        let current_week = self.get_current_week();
        let base_token_id = self.get_base_token_id();

        if payment.token_nonce != 0 {
            self.try_burn_locked_token(&payment);

            self.accumulated_fees(current_week, &payment.token_identifier)
                .update(|amt| *amt += &payment.amount);
        } else if payment.token_identifier == base_token_id {
            self.burn_part_of_base_token(&mut payment);

            self.accumulated_fees(current_week, &payment.token_identifier)
                .update(|amt| *amt += &payment.amount);
        } else {
            self.all_accumulated_tokens(&payment.token_identifier)
                .update(|acc_tokens| *acc_tokens += &payment.amount);
        }

        let caller = self.blockchain().get_caller();
        self.emit_deposit_swap_fees_event(&caller, current_week, &payment);
    }

    fn try_burn_locked_token(&self, payment: &EsdtTokenPayment) {
        let locked_token_id = self.get_locked_token_id();
        require!(
            payment.token_identifier == locked_token_id,
            "Only locked token accepted as SFT/NFT/MetaESDT"
        );

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );
    }

    fn burn_part_of_base_token(&self, payment: &mut EsdtTokenPayment) {
        let burn_percent = self.base_token_burn_percent().get();
        if burn_percent == 0 {
            return;
        }

        let burn_amount = &payment.amount * burn_percent / MAX_PENALTY_PERCENTAGE;
        if burn_amount == 0 {
            return;
        }

        self.send()
            .esdt_local_burn(&payment.token_identifier, 0, &burn_amount);

        payment.amount -= burn_amount;
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

    #[storage_mapper("baseTokenBurnPercent")]
    fn base_token_burn_percent(&self) -> SingleValueMapper<Percent>;
}
