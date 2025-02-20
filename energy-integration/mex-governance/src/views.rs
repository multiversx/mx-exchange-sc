multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    config::{self, FarmVoteView},
    events,
};

#[multiversx_sc::module]
pub trait ViewsModule:
    config::ConfigModule
    + events::EventsModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[view(getAllWeekEmissions)]
    fn get_all_week_emissions(&self, week: usize) -> MultiValueEncoded<FarmVoteView<Self::Api>> {
        let emission_rate = self.emission_rate_for_week(week).get();
        let total_votes = self.total_energy_voted(week).get();

        let mut result = MultiValueEncoded::new();
        for farm_id in self.voted_farms_for_week(week).iter() {
            let farm_address_opt = self.farm_ids().get_address(farm_id);
            if farm_address_opt.is_none() {
                continue;
            }
            let farm_address = unsafe { farm_address_opt.unwrap_unchecked() };

            let farm_votes = self.farm_votes_for_week(farm_id, week).get();
            let farm_emission = &emission_rate * &farm_votes / &total_votes;

            result.push(FarmVoteView {
                farm_address,
                farm_emission,
            });
        }

        result
    }
}
