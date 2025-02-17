multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    events, EMISSION_RATE_ZERO, FARM_NOT_WHITELISTED, INVALID_FARM_ADDRESS,
    WEEK_ALREADY_INITIALIZED,
};

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]

pub struct FarmEmission<M: ManagedTypeApi> {
    pub farm_address: ManagedAddress<M>,
    pub farm_emission: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConfigModule:
    events::EventsModule + energy_query::EnergyQueryModule + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(initializeFirstWeek)]
    fn initialize_first_week(
        &self,
        farm_allocations: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        let current_week = self.get_current_week();
        let emission_rate_for_week_mapper = self.emission_rate_for_week(current_week);
        require!(
            emission_rate_for_week_mapper.is_empty(),
            WEEK_ALREADY_INITIALIZED
        );

        let emission_rate = self.reference_emission_rate().get();
        self.emission_rate_for_week(current_week)
            .set(&emission_rate);

        let mut total_amount = BigUint::zero();
        for allocation in farm_allocations {
            let (farm_address, amount) = allocation.into_tuple();

            self.farm_votes_for_week(&farm_address, current_week)
                .set(&amount);

            self.whitelisted_farms().insert(farm_address.clone());
            self.voted_farms_for_week(current_week).insert(farm_address);

            total_amount += amount;
        }

        self.total_energy_voted(current_week).set(&total_amount);
    }

    // TODO
    // Better define the blacklist behavior
    #[only_owner]
    #[endpoint(blacklistFarm)]
    fn blacklist_farm(&self, farm_address: ManagedAddress) {
        require!(
            self.whitelisted_farms().contains(&farm_address),
            FARM_NOT_WHITELISTED
        );

        self.whitelisted_farms().swap_remove(&farm_address);
        self.blacklisted_farms().insert(farm_address.clone());
        // self.farm_blacklisted_event(&farm_address);
    }

    #[only_owner]
    #[endpoint(whitelistFarm)]
    fn whitelist_farm(&self, farm_addresses: MultiValueEncoded<ManagedAddress>) {
        for farm_address in farm_addresses {
            require!(
                self.blockchain().is_smart_contract(&farm_address),
                INVALID_FARM_ADDRESS
            );
            self.whitelisted_farms().insert(farm_address.clone());
        }
    }

    #[only_owner]
    #[endpoint(setReferenceEmissionRate)]
    fn set_reference_emission_rate(&self, new_rate: BigUint) {
        require!(new_rate > 0, EMISSION_RATE_ZERO);
        let old_rate = self.reference_emission_rate().get();
        self.reference_emission_rate().set(&new_rate);

        self.emit_reference_emission_rate_event(old_rate, new_rate);
    }

    // Weekly storages
    #[storage_mapper("emissionRateForWeek")]
    fn emission_rate_for_week(&self, week: usize) -> SingleValueMapper<BigUint>;

    #[storage_mapper("votedFarmsForWeek")]
    fn voted_farms_for_week(&self, week: usize) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("farmVotesForPeriod")]
    fn farm_votes_for_week(
        &self,
        farm_address: &ManagedAddress,
        week: usize,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalEnergyVoted")]
    fn total_energy_voted(&self, week: usize) -> SingleValueMapper<BigUint>;

    #[storage_mapper("usersVotedInWeek")]
    fn users_voted_in_week(&self, week: usize) -> WhitelistMapper<ManagedAddress>;

    // General storages
    #[storage_mapper("votingWeek")]
    fn voting_week(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("referenceEmissionRate")]
    fn reference_emission_rate(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("whitelistedFarms")]
    fn whitelisted_farms(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("blacklistedFarms")]
    fn blacklisted_farms(&self) -> UnorderedSetMapper<ManagedAddress>;
}
