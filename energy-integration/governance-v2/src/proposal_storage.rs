elrond_wasm::imports!();

use crate::proposal::{GovernanceProposal, ProposalId};

#[elrond_wasm::module]
pub trait ProposalStorageModule {
    #[storage_mapper("proposals")]
    fn proposals(&self) -> VecMapper<GovernanceProposal<Self::Api>>;

    #[storage_mapper("requiredPaymentsForProposal")]
    fn required_payments_for_proposal(
        &self,
        proposal_id: ProposalId,
    ) -> SingleValueMapper<ManagedVec<EsdtTokenPayment<Self::Api>>>;

    #[storage_mapper("paymentsDepositor")]
    fn payments_depositor(&self, proposal_id: ProposalId) -> SingleValueMapper<ManagedAddress>;

    // Not stored under "proposals", as that would require deserializing the whole struct
    #[storage_mapper("proposalStartBlock")]
    fn proposal_start_block(&self, proposal_id: ProposalId) -> SingleValueMapper<u64>;

    #[storage_mapper("proposalQueueBlock")]
    fn proposal_queue_block(&self, proposal_id: ProposalId) -> SingleValueMapper<u64>;

    #[storage_mapper("governance:userVotedProposals")]
    fn user_voted_proposals(&self, user: &ManagedAddress) -> UnorderedSetMapper<ProposalId>;

    #[view(getTotalVotes)]
    #[storage_mapper("totalVotes")]
    fn total_votes(&self, proposal_id: ProposalId) -> SingleValueMapper<BigUint>;

    #[view(getTotalDownvotes)]
    #[storage_mapper("totalDownvotes")]
    fn total_downvotes(&self, proposal_id: ProposalId) -> SingleValueMapper<BigUint>;
}
