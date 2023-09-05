multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::proposal::{GovernanceProposal, ProposalId};

#[derive(TypeAbi, TopEncode, TopDecode)]
pub enum VoteType {
    UpVote,
    DownVote,
    DownVetoVote,
    AbstainVote,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct ProposalVotes<M: ManagedTypeApi> {
    pub up_votes: BigUint<M>,
    pub down_votes: BigUint<M>,
    pub down_veto_votes: BigUint<M>,
    pub abstain_votes: BigUint<M>,
    pub quorum: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for ProposalVotes<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ManagedTypeApi> ProposalVotes<M> {
    pub fn new() -> Self {
        ProposalVotes {
            up_votes: BigUint::zero(),
            down_votes: BigUint::zero(),
            down_veto_votes: BigUint::zero(),
            abstain_votes: BigUint::zero(),
            quorum: BigUint::zero(),
        }
    }
    pub fn get_total_votes(&self) -> BigUint<M> {
        &self.up_votes + &self.down_votes + &self.down_veto_votes + &self.abstain_votes
    }
    pub fn get_up_votes_percentage(&self) -> BigUint<M> {
        let total_votes = self.get_total_votes();
        &self.up_votes / &total_votes
    }
    pub fn get_down_votes_percentage(&self) -> BigUint<M> {
        let total_votes = self.get_total_votes();
        &self.down_votes / &total_votes
    }
    pub fn get_down_veto_votes_percentage(&self) -> BigUint<M> {
        let total_votes = self.get_total_votes();
        &self.down_veto_votes / &total_votes
    }
    pub fn get_abstain_votes_percentage(&self) -> BigUint<M> {
        let total_votes = self.get_total_votes();
        &self.abstain_votes / &total_votes
    }
}

#[multiversx_sc::module]
pub trait ProposalStorageModule {
    fn clear_proposal(&self, proposal_id: ProposalId) {
        self.proposals().clear_entry(proposal_id);
        self.proposal_votes(proposal_id).clear();
    }

    #[view(getProposals)]
    #[storage_mapper("proposals")]
    fn proposals(&self) -> VecMapper<GovernanceProposal<Self::Api>>;

    #[view(getUserVotedProposals)]
    #[storage_mapper("userVotedProposals")]
    fn user_voted_proposals(&self, user: &ManagedAddress) -> UnorderedSetMapper<ProposalId>;

    #[view(getProposalVotes)]
    #[storage_mapper("proposalVotes")]
    fn proposal_votes(
        &self,
        proposal_id: ProposalId,
    ) -> SingleValueMapper<ProposalVotes<Self::Api>>;
}
