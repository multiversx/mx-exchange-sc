multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::proposal::{GovernanceProposal, ProposalId};

#[multiversx_sc::module]
pub trait EventsModule {
    #[event("proposalCreated")]
    fn proposal_created_event(
        &self,
        #[indexed] proposal_id: usize,
        #[indexed] proposer: &ManagedAddress,
        #[indexed] start_block: u64,
        proposal: &GovernanceProposal<Self::Api>,
    );

    #[event("upVoteCast")]
    fn up_vote_cast_event(
        &self,
        #[indexed] up_voter: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        nr_votes: &BigUint,
    );

    #[event("downVoteCast")]
    fn down_vote_cast_event(
        &self,
        #[indexed] down_voter: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        nr_downvotes: &BigUint,
    );

    #[event("downVetoVoteCast")]
    fn down_veto_vote_cast_event(
        &self,
        #[indexed] down_veto_voter: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        nr_downvotes: &BigUint,
    );

    #[event("abstainVoteCast")]
    fn abstain_vote_cast_event(
        &self,
        #[indexed] abstain_voter: &ManagedAddress,
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
        payment: &EsdtTokenPayment,
    );

    #[event("userClaimDepositedTokens")]
    fn user_claim_deposited_tokens_event(
        &self,
        #[indexed] address: &ManagedAddress,
        #[indexed] proposal_id: ProposalId,
        payments: &ManagedVec<EsdtTokenPayment>,
    );
}
