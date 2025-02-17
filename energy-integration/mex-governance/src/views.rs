multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{config, events};

#[multiversx_sc::module]
pub trait ViewsModule:
    config::ConfigModule
    + events::EventsModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[view(getFarmCurrentWeekEmission)]
    fn get_farm_current_week_emission(&self) -> BigUint {
        let current_week = self.get_current_week();
        let farm_address = self.blockchain().get_caller();

        let emission_rate = self.emission_rate_for_week(current_week).get();
        let total_votes = self.total_energy_voted(current_week).get();
        let farm_votes = self.farm_votes_for_week(&farm_address, current_week).get();

        emission_rate * farm_votes / total_votes
    }

    #[view(getAllWeekEmissions)]
    fn get_all_week_emissions(&self, week: usize) -> MultiValueEncoded<FarmEmission<Self::Api>> {
        let emission_rate = self.emission_rate_for_week(week).get();
        let total_votes = self.total_energy_voted(week).get();

        let mut result = MultiValueEncoded::new();
        for farm_address in self.voted_farms_for_week(week).iter() {
            let farm_votes = self.farm_votes_for_week(&farm_address, week).get();

            let farm_emission = &emission_rate * &farm_votes / &total_votes;
            result.push(FarmEmission {
                farm_address,
                farm_emission,
            });
        }

        result
    }
}
