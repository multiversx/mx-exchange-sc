use crate::config::FarmEmission;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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
}
