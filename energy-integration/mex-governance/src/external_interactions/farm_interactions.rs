multiversx_sc::imports!();

use multiversx_sc::storage::StorageKey;

use crate::config::FarmEmission;

pub static PRODUCE_REWARDS_ENABLED_STORAGE_KEY: &[u8] = b"produce_rewards_enabled";

#[multiversx_sc::module]
pub trait FarmInteractionsModule:
    crate::config::ConfigModule
    + crate::events::EventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
{
    #[only_owner]
    #[endpoint(resetFarmEmissions)]
    fn reset_farm_emissions(&self, farms: MultiValueEncoded<ManagedAddress>) {
        for farm in farms.into_iter() {
            self.farm_proxy(farm)
                .end_produce_rewards_endpoint()
                .execute_on_dest_context::<()>();
        }
    }

    #[endpoint(setFarmEmissions)]
    fn set_farm_emissions(&self) {
        let current_week = self.get_current_week();
        let previous_week = current_week - 1; // Current week starts from 1, so we shouldn't overflow
        if previous_week > 0 {
            self.reset_previous_farms_emissions(previous_week);
        }

        let total_emission_rate = self.emission_rate_for_week(current_week).get();
        let total_votes = self.total_energy_voted(current_week).get();

        if total_votes == 0 {
            return;
        }

        let current_farm_emissions = self.farm_emissions_for_week(current_week).get();

        if current_farm_emissions.is_empty() {
            return;
        }

        self.distribute_emissions(
            current_week,
            current_farm_emissions,
            total_emission_rate,
            total_votes,
        );
    }

    fn distribute_emissions(
        &self,
        week: usize,
        farm_emissions: ManagedVec<FarmEmission<Self::Api>>,
        total_emission_rate: BigUint,
        total_votes: BigUint,
    ) {
        let redistributed_votes = self.redistributed_votes_for_week(week).get();
        let top_farms_total_votes = &total_votes - &redistributed_votes;

        if top_farms_total_votes == 0 {
            return;
        }

        let mut new_farm_emissions = ManagedVec::new();
        let mut total_distributed = BigUint::zero();

        let farms_to_process = farm_emissions.len() - 1;
        for i in 0..farms_to_process {
            let farm = farm_emissions.get(i);

            // Simple calculation - distribute total emissions only among top farms
            // Redistributed amount is applied implicitly
            let farm_emission = &total_emission_rate * &farm.farm_emission / &top_farms_total_votes;

            total_distributed += &farm_emission;

            self.set_farm_emission(&farm.farm_address, &farm_emission);

            new_farm_emissions.push(FarmEmission {
                farm_address: farm.farm_address,
                farm_emission: farm_emission.clone(),
            });
        }

        require!(
            total_distributed <= total_emission_rate,
            "Total distributed emissions exceed the total emission rate"
        );

        if !farm_emissions.is_empty() {
            let last_farm = farm_emissions.get(farm_emissions.len() - 1);
            let last_farm_emission = &total_emission_rate - &total_distributed;

            self.set_farm_emission(&last_farm.farm_address, &last_farm_emission);

            new_farm_emissions.push(FarmEmission {
                farm_address: last_farm.farm_address,
                farm_emission: last_farm_emission,
            });
        }

        self.emit_farm_emissions_event(week, new_farm_emissions);
    }

    fn reset_previous_farms_emissions(&self, week: usize) {
        let previous_farms = self.farm_emissions_for_week(week).get();

        for farm_emission in previous_farms.iter() {
            self.farm_proxy(farm_emission.farm_address.clone())
                .end_produce_rewards_endpoint()
                .execute_on_dest_context::<()>();
        }
    }

    fn set_farm_emission(&self, farm_address: &ManagedAddress, new_farm_emissions: &BigUint) {
        self.farm_proxy(farm_address.clone())
            .set_per_block_rewards_endpoint(new_farm_emissions.clone())
            .execute_on_dest_context::<()>();

        let produce_rewards_enabled = self.get_farm_produce_rewards_enabled(farm_address.clone());

        if !produce_rewards_enabled {
            self.farm_proxy(farm_address.clone())
                .start_produce_rewards_endpoint()
                .execute_on_dest_context::<()>();
        }
    }

    fn get_farm_produce_rewards_enabled(&self, farm_address: ManagedAddress) -> bool {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            farm_address,
            StorageKey::new(PRODUCE_REWARDS_ENABLED_STORAGE_KEY),
        )
        .get()
    }

    #[proxy]
    fn farm_proxy(&self, to: ManagedAddress) -> farm_with_locked_rewards::Proxy<Self::Api>;
}
