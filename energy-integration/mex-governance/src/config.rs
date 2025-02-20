multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use week_timekeeping::Week;

use crate::{
    events, EMISSION_RATE_ZERO, FARM_NOT_WHITELISTED, INVALID_ESDT_IDENTIFIER, INVALID_FARM_ADDRESS,
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

pub struct FarmVoteView<M: ManagedTypeApi> {
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
    events::EventsModule + energy_query::EnergyQueryModule + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(whitelistFarms)]
    fn whitelist_farms(&self, farms: MultiValueEncoded<ManagedAddress>) {
        let farms_mapper = self.farm_ids();

        for farm_address in farms {
            require!(
                self.blockchain().is_smart_contract(&farm_address),
                INVALID_FARM_ADDRESS
            );

            let new_id = farms_mapper.get_id_or_insert(&farm_address);

            require!(
                !self.blacklisted_farms().contains(&new_id),
                FARM_NOT_WHITELISTED
            );

            self.whitelisted_farms().insert(new_id);
        }
    }

    #[only_owner]
    #[endpoint(removeWhitelistFarm)]
    fn remove_whitelist_farm(&self, farms: MultiValueEncoded<ManagedAddress>) {
        for farm_address in farms {
            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);
            require!(
                self.whitelisted_farms().swap_remove(&farm_id),
                FARM_NOT_WHITELISTED
            );
        }
    }

    #[only_owner]
    #[endpoint(blacklistFarm)]
    fn blacklist_farm(&self, farms: MultiValueEncoded<ManagedAddress>) {
        for farm_address in farms {
            let farm_id = self.farm_ids().get_id_non_zero(&farm_address);
            require!(
                self.whitelisted_farms().swap_remove(&farm_id),
                FARM_NOT_WHITELISTED
            );

            self.blacklisted_farms().insert(farm_id);
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

    #[storage_mapper("farmIncentiveForWeek")]
    fn farm_incentive_for_week(&self, farm_id: AddressId, week: Week)
        -> SingleValueMapper<BigUint>;

    #[storage_mapper("usersVotedInWeek")]
    fn user_votes_in_week(
        &self,
        user_id: AddressId,
        week: Week,
    ) -> SingleValueMapper<ManagedVec<FarmVote<Self::Api>>>;

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
