multiversx_sc::imports!();

use crate::{config::FarmEmission, errors::FARM_NOT_FOUND};

#[multiversx_sc::module]
pub trait FarmInteractionsModule:
    crate::config::ConfigModule
    + crate::events::EventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
{
    #[endpoint(setFarmEmissions)]
    fn set_farm_emissions(&self) {
        let current_week = self.get_current_week();
        let emission_rate = self.emission_rate_for_week(current_week).get();
        let total_votes = self.total_energy_voted(current_week).get();

        let mut farm_emissions = ManagedVec::new();

        for farm_id in self.voted_farms_for_week(current_week).iter() {
            let farm_address_opt = self.farm_ids().get_address(farm_id);
            require!(farm_address_opt.is_some(), FARM_NOT_FOUND);

            let farm_address = unsafe { farm_address_opt.unwrap_unchecked() };

            let farm_votes = self.farm_votes_for_week(farm_id, current_week).get();

            let farm_emission = &emission_rate * &farm_votes / &total_votes;
            self.farm_proxy(farm_address.clone())
                .set_per_block_rewards_endpoint(farm_emission.clone())
                .execute_on_dest_context::<()>();

            farm_emissions.push(FarmEmission {
                farm_address,
                farm_emission,
            });
        }

        // Add event emission
        self.emit_farm_emissions_event(current_week, farm_emissions);
    }

    #[proxy]
    fn farm_proxy(&self, to: ManagedAddress) -> farm_with_locked_rewards::Proxy<Self::Api>;
}
