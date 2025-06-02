multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use week_timekeeping::Week;

use crate::errors::{EMISSION_RATE_ZERO, INVALID_ESDT_IDENTIFIER};

/// Constant for maximum number of farms that receive rewards
pub const MAX_REWARDED_FARMS: usize = 25;

/// Maximum number of farms a user can vote for in a single transaction
pub const MAX_FARMS_PER_VOTE: usize = 10;

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

pub struct FarmVote<M: ManagedTypeApi> {
    pub farm_id: AddressId,
    pub vote_amount: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConfigModule:
    crate::events::EventsModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(blacklistFarm)]
    fn blacklist_farm(&self, farms: MultiValueEncoded<ManagedAddress>) {
        let mut blacklisted_farms = ManagedVec::new();

        for farm_address in farms {
            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);
            self.blacklisted_farms().insert(farm_id);
            blacklisted_farms.push(farm_address);
        }

        self.emit_blacklist_farm_event(blacklisted_farms);
    }

    #[only_owner]
    #[endpoint(removeBlacklistFarm)]
    fn remove_blacklist_farm(&self, farms: MultiValueEncoded<ManagedAddress>) {
        let mut remove_from_blacklist_farms = ManagedVec::new();
        for farm_address in farms {
            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);
            self.blacklisted_farms().swap_remove(&farm_id);
            remove_from_blacklist_farms.push(farm_address);
        }

        self.emit_remove_blacklist_farm_event(remove_from_blacklist_farms);
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
        let old_token = self.incentive_token().get();
        self.incentive_token().set(&token_id);

        self.emit_set_incentive_token_event(old_token, token_id);
    }

    #[view(isAddressBlacklisted)]
    fn is_address_blacklisted(&self, address: &ManagedAddress) -> bool {
        let address_id = self.farm_ids().get_id_non_zero(address);
        self.blacklisted_farms().contains(&address_id)
    }

    // Weekly storages
    #[view(getEmissionRateForWeek)]
    #[storage_mapper("emissionRateForWeek")]
    fn emission_rate_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getVotedFarmsForWeek)]
    #[storage_mapper("votedFarmsForWeek")]
    fn voted_farms_for_week(&self, week: Week) -> UnorderedSetMapper<AddressId>;

    #[view(getFarmVotesForWeek)]
    #[storage_mapper("farmVotesForWeek")]
    fn farm_votes_for_week(&self, farm_id: AddressId, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getTotalEnergyVoted)]
    #[storage_mapper("totalEnergyVoted")]
    fn total_energy_voted(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[view(getFarmIncentiveForWeek)]
    #[storage_mapper("farmIncentiveForWeek")]
    fn farm_incentive_for_week(&self, farm_id: AddressId, week: Week)
        -> SingleValueMapper<BigUint>;

    #[view(getUsersVotedInWeek)]
    #[storage_mapper("usersVotedInWeek")]
    fn user_votes_in_week(
        &self,
        user_id: AddressId,
        week: Week,
    ) -> SingleValueMapper<ManagedVec<FarmVote<Self::Api>>>;

    #[view(getFarmEmissionsForWeek)]
    #[storage_mapper("farmEmissionsForWeek")]
    fn farm_emissions_for_week(
        &self,
        week: Week,
    ) -> SingleValueMapper<ManagedVec<FarmEmission<Self::Api>>>;

    #[view(getRedistributedVotesForWeek)]
    #[storage_mapper("redistributedVotesForWeek")]
    fn redistributed_votes_for_week(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("topFarmsWhitelist")]
    fn top_farms_whitelist(&self, week: Week) -> WhitelistMapper<ManagedAddress>;

    // General storages

    #[storage_mapper("farmIds")]
    fn farm_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("userIds")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getVotingWeek)]
    #[storage_mapper("votingWeek")]
    fn voting_week(&self) -> SingleValueMapper<Week>;

    #[view(getLastEmissionWeek)]
    #[storage_mapper("lastEmissionsWeek")]
    fn last_emission_week(&self) -> SingleValueMapper<Week>;

    #[view(getReferenceEmissionRate)]
    #[storage_mapper("referenceEmissionRate")]
    fn reference_emission_rate(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("blacklistedFarms")]
    fn blacklisted_farms(&self) -> UnorderedSetMapper<AddressId>;

    #[view(getIncentiveToken)]
    #[storage_mapper("incentiveToken")]
    fn incentive_token(&self) -> SingleValueMapper<TokenIdentifier>;
}
