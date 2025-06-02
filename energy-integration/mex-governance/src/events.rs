multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::config::FarmEmission;

#[derive(TypeAbi, TopEncode)]
pub struct ReferenceEmissionRateEvent<M: ManagedTypeApi> {
    old_reference_emission_rate: BigUint<M>,
    new_reference_emission_rate: BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct VoteEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    voting_week: usize,
    farm_votes: ManagedVec<M, FarmEmission<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct FarmEmissionsEvent<M: ManagedTypeApi> {
    pub week: usize,
    pub farm_emissions: ManagedVec<M, FarmEmission<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct IncentivizeFarmEvent<M: ManagedTypeApi> {
    pub caller: ManagedAddress<M>,
    pub token_identifier: TokenIdentifier<M>,
    pub farm_incentives: ManagedVec<M, FarmIncentiveView<M>>,
}

#[derive(TypeAbi, TopEncode, ManagedVecItem, NestedEncode, NestedDecode, Clone)]
pub struct FarmIncentiveView<M: ManagedTypeApi> {
    pub farm_address: ManagedAddress<M>,
    pub amount: BigUint<M>,
    pub week: usize,
}

#[derive(TypeAbi, TopEncode)]
pub struct ClaimIncentiveEvent<M: ManagedTypeApi> {
    pub caller: ManagedAddress<M>,
    pub week: usize,
    pub claimed_amounts: ManagedVec<M, ClaimedIncentiveView<M>>,
}

#[derive(TypeAbi, TopEncode, ManagedVecItem, NestedEncode, NestedDecode, Clone)]
pub struct ClaimedIncentiveView<M: ManagedTypeApi> {
    pub farm_address: ManagedAddress<M>,
    pub amount: BigUint<M>,
}

#[derive(TypeAbi, TopEncode)]
pub struct WhitelistFarmsEvent<M: ManagedTypeApi> {
    pub farms: ManagedVec<M, ManagedAddress<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct RemoveWhitelistFarmEvent<M: ManagedTypeApi> {
    pub farms: ManagedVec<M, ManagedAddress<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct BlacklistFarmEvent<M: ManagedTypeApi> {
    pub farms: ManagedVec<M, ManagedAddress<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct RemoveBlacklistFarmEvent<M: ManagedTypeApi> {
    pub farms: ManagedVec<M, ManagedAddress<M>>,
}

#[derive(TypeAbi, TopEncode)]
pub struct SetIncentiveTokenEvent<M: ManagedTypeApi> {
    pub old_token: TokenIdentifier<M>,
    pub new_token: TokenIdentifier<M>,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_vote_event(&self, voting_week: usize, votes: ManagedVec<FarmEmission<Self::Api>>) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.vote_event(
            caller.clone(),
            voting_week,
            block,
            epoch,
            timestamp,
            VoteEvent {
                caller,
                voting_week,
                farm_votes: votes,
            },
        );
    }

    fn emit_reference_emission_rate_event(
        &self,
        old_reference_emission_rate: BigUint,
        new_reference_emission_rate: BigUint,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.reference_emission_rate_event(
            caller,
            block,
            epoch,
            timestamp,
            ReferenceEmissionRateEvent {
                old_reference_emission_rate,
                new_reference_emission_rate,
            },
        );
    }

    fn emit_farm_emissions_event(
        &self,
        week: usize,
        farm_emissions: ManagedVec<FarmEmission<Self::Api>>,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.farm_emissions_event(
            caller,
            week,
            block,
            epoch,
            timestamp,
            FarmEmissionsEvent {
                week,
                farm_emissions,
            },
        );
    }

    fn emit_incentivize_farm_event(
        &self,
        token_identifier: TokenIdentifier,
        farm_incentives: ManagedVec<FarmIncentiveView<Self::Api>>,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.incentivize_farm_event(
            caller.clone(),
            block,
            epoch,
            timestamp,
            IncentivizeFarmEvent {
                caller,
                token_identifier,
                farm_incentives,
            },
        );
    }

    fn emit_claim_incentive_event(
        &self,
        week: usize,
        claimed_amounts: ManagedVec<ClaimedIncentiveView<Self::Api>>,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.claim_incentive_event(
            caller.clone(),
            week,
            block,
            epoch,
            timestamp,
            ClaimIncentiveEvent {
                caller,
                week,
                claimed_amounts,
            },
        );
    }

    fn emit_blacklist_farm_event(&self, farms: ManagedVec<ManagedAddress<Self::Api>>) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.blacklist_farm_event(
            caller,
            block,
            epoch,
            timestamp,
            BlacklistFarmEvent { farms },
        );
    }

    fn emit_remove_blacklist_farm_event(&self, farms: ManagedVec<ManagedAddress<Self::Api>>) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.remove_blacklist_farm_event(
            caller,
            block,
            epoch,
            timestamp,
            RemoveBlacklistFarmEvent { farms },
        );
    }

    fn emit_set_incentive_token_event(
        &self,
        old_token: TokenIdentifier,
        new_token: TokenIdentifier,
    ) {
        let caller = self.blockchain().get_caller();
        let block = self.blockchain().get_block_nonce();
        let epoch = self.blockchain().get_block_epoch();
        let timestamp = self.blockchain().get_block_timestamp();
        self.set_incentive_token_event(
            caller,
            block,
            epoch,
            timestamp,
            SetIncentiveTokenEvent {
                old_token,
                new_token,
            },
        );
    }

    #[event("voteEvent")]
    fn vote_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] week: usize,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        vote_event: VoteEvent<Self::Api>,
    );

    #[event("referenceEmissionRateEvent")]
    fn reference_emission_rate_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        reference_emission_rate_event: ReferenceEmissionRateEvent<Self::Api>,
    );

    #[event("farmEmissionsEvent")]
    fn farm_emissions_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] week: usize,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: FarmEmissionsEvent<Self::Api>,
    );

    #[event("incentivizeFarmEvent")]
    fn incentivize_farm_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: IncentivizeFarmEvent<Self::Api>,
    );

    #[event("claimIncentiveEvent")]
    fn claim_incentive_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] week: usize,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: ClaimIncentiveEvent<Self::Api>,
    );

    #[event("blacklistFarmEvent")]
    fn blacklist_farm_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: BlacklistFarmEvent<Self::Api>,
    );

    #[event("removeBlacklistFarmEvent")]
    fn remove_blacklist_farm_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: RemoveBlacklistFarmEvent<Self::Api>,
    );

    #[event("setIncentiveTokenEvent")]
    fn set_incentive_token_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        event: SetIncentiveTokenEvent<Self::Api>,
    );
}
