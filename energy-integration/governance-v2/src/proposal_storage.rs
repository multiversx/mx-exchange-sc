elrond_wasm::imports!();
elrond_wasm::derive_imports!();

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
}

impl<M: ManagedTypeApi> ProposalVotes<M> {
    pub fn new(
        up_votes: BigUint<M>,
        down_votes: BigUint<M>,
        down_veto_votes: BigUint<M>,
        abstain_votes: BigUint<M>,
    ) -> Self {
        ProposalVotes {
            up_votes,
            down_votes,
            down_veto_votes,
            abstain_votes,
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

    #[view(getProposalVotes)]
    #[storage_mapper("proposalVotes")]
    fn proposal_votes(
        &self,
        proposal_id: ProposalId,
    ) -> SingleValueMapper<ProposalVotes<Self::Api>>;
}
