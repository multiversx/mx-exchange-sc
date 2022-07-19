elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::proposal::{GovernanceProposal, ProposalId};

#[elrond_wasm::module]
pub trait EventsModule {
    #[event("proposalCreated")]
    fn proposal_created_event(
        &self,
        #[indexed] proposal_id: usize,
        #[indexed] proposer: &ManagedAddress,
        #[indexed] start_block: u64,
        proposal: &GovernanceProposal<Self::Api>,
    );

    #[event("voteCast")]
    fn vote_cast_event(
        &self,
        #[indexed] voter: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        nr_votes: &BigUint,
    );

    #[event("downvoteCast")]
    fn downvote_cast_event(
        &self,
        #[indexed] downvoter: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        nr_downvotes: &BigUint,
    );

    #[event("proposalCanceled")]
    fn proposal_canceled_event(&self, #[indexed] proposal_id: ProposalId);

    #[event("proposalQueued")]
    fn proposal_queued_event(
        &self,
        #[indexed] proposal_id: ProposalId,
        #[indexed] queued_block: u64,
    );

    #[event("proposalExecuted")]
    fn proposal_executed_event(&self, #[indexed] proposal_id: ProposalId);

    #[event("userDeposit")]
    fn user_deposit_event(
        &self,
        #[indexed] address: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    );
}
