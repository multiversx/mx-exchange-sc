multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use week_timekeeping::Week;

use crate::{
    events, EMISSION_RATE_ZERO, FARM_NOT_WHITELISTED, INVALID_ESDT_IDENTIFIER,
    INVALID_FARM_ADDRESS, WEEK_ALREADY_INITIALIZED,
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
    pub farm_id: AddressId,
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
        farm_allocations: MultiValueEncoded<MultiValue2<AddressId, BigUint>>,
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
            let (farm_id, amount) = allocation.into_tuple();

            self.farm_votes_for_week(farm_id, current_week).set(&amount);

            self.whitelisted_farms().insert(farm_id);
            self.voted_farms_for_week(current_week).insert(farm_id);

            total_amount += amount;
        }

        self.total_energy_voted(current_week).set(&total_amount);
    }

    #[only_owner]
    #[endpoint(whitelistFarms)]
    fn whitelist_farms(&self, farms: MultiValueEncoded<ManagedAddress>) -> MultiValueEncoded<u64> {
        let farms_mapper = self.farm_ids();

        let mut farm_ids = MultiValueEncoded::new();
        for farm_address in farms {
            require!(
                self.blockchain().is_smart_contract(&farm_address),
                INVALID_FARM_ADDRESS
            );

            let new_id = farms_mapper.insert_new(&farm_address);
            farm_ids.push(new_id);

            self.whitelisted_farms().insert(new_id);
        }

        farm_ids
    }

    #[only_owner]
    #[endpoint(blacklistFarm)]
    fn blacklist_farm(&self, farm_id: AddressId) {
        require!(
            self.whitelisted_farms().contains(&farm_id),
            FARM_NOT_WHITELISTED
        );

        self.whitelisted_farms().swap_remove(&farm_id);
        self.blacklisted_farms().insert(farm_id);
    }

    #[only_owner]
    #[endpoint(setReferenceEmissionRate)]
    fn set_reference_emission_rate(&self, new_rate: BigUint) {
        require!(new_rate > 0, EMISSION_RATE_ZERO);
        let old_rate = self.reference_emission_rate().get();
        self.reference_emission_rate().set(&new_rate);

        self.emit_reference_emission_rate_event(old_rate, new_rate);
    }

    #[only_owner]
    #[endpoint(setIncentiveToken)]
    fn set_incentive_token(&self, token_id: TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), INVALID_ESDT_IDENTIFIER);
        self.incentive_token().set(&token_id);
    }

    // Weekly storages
    #[storage_mapper("emissionRateForWeek")]
    fn emission_rate_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("votedFarmsForWeek")]
    fn voted_farms_for_week(&self, week: Week) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("farmVotesForPeriod")]
    fn farm_votes_for_week(&self, farm_id: AddressId, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalEnergyVoted")]
    fn total_energy_voted(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("usersVotedInWeek")]
    fn users_voted_in_week(&self, week: Week) -> WhitelistMapper<AddressId>;

    // General storages

    #[storage_mapper("farmIds")]
    fn farm_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("userIds")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("votingWeek")]
    fn voting_week(&self) -> SingleValueMapper<Week>;

    #[storage_mapper("referenceEmissionRate")]
    fn reference_emission_rate(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("whitelistedFarms")]
    fn whitelisted_farms(&self) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("blacklistedFarms")]
    fn blacklisted_farms(&self) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("incentiveToken")]
    fn incentive_token(&self) -> SingleValueMapper<TokenIdentifier>;
}
