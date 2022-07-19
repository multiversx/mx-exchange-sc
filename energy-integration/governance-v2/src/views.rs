elrond_wasm::imports!();

use crate::proposal::{
    GovernanceAction, GovernanceProposalStatus, ProposalId, MAX_GOVERNANCE_PROPOSAL_ACTIONS,
};

#[elrond_wasm::module]
pub trait ViewsModule:
    crate::proposal_storage::ProposalStorageModule
    + crate::configurable::ConfigurablePropertiesModule
    + energy_query_module::EnergyQueryModule
{
    #[view(getProposalStatus)]
    fn get_proposal_status(&self, proposal_id: ProposalId) -> GovernanceProposalStatus {
        if !self.proposal_exists(proposal_id) {
            return GovernanceProposalStatus::None;
        }

        let queue_block = self.proposal_queue_block(proposal_id).get();
        if queue_block > 0 {
            return GovernanceProposalStatus::Queued;
        }

        let current_block = self.blockchain().get_block_nonce();
        let proposal_block = self.proposal_start_block(proposal_id).get();
        let voting_delay = self.voting_delay_in_blocks().get();
        let voting_period = self.voting_period_in_blocks().get();

        let voting_start = proposal_block + voting_delay;
        let voting_end = voting_start + voting_period;

        if current_block < voting_start {
            return GovernanceProposalStatus::Pending;
        }
        if current_block >= voting_start && current_block < voting_end {
            return GovernanceProposalStatus::Active;
        }

        if self.quorum_reached(proposal_id) {
            GovernanceProposalStatus::Succeeded
        } else {
            GovernanceProposalStatus::Defeated
        }
    }

    fn quorum_reached(&self, proposal_id: ProposalId) -> bool {
        let total_votes = self.total_votes(proposal_id).get();
        let total_downvotes = self.total_downvotes(proposal_id).get();
        let quorum = self.quorum().get();

        &total_votes + &total_downvotes >= quorum && total_votes > total_downvotes
    }

    #[view(getProposer)]
    fn get_proposer(&self, proposal_id: ProposalId) -> OptionalValue<ManagedAddress> {
        if !self.proposal_exists(proposal_id) {
            return OptionalValue::None;
        }

        OptionalValue::Some(self.proposals().get(proposal_id).proposer)
    }

    #[view(getProposalDescription)]
    fn get_proposal_description(&self, proposal_id: ProposalId) -> OptionalValue<ManagedBuffer> {
        if !self.proposal_exists(proposal_id) {
            return OptionalValue::None;
        }

        OptionalValue::Some(self.proposals().get(proposal_id).description)
    }

    #[view(getProposalActions)]
    fn get_proposal_actions(
        &self,
        proposal_id: ProposalId,
    ) -> ArrayVec<GovernanceAction<Self::Api>, MAX_GOVERNANCE_PROPOSAL_ACTIONS> {
        if !self.proposal_exists(proposal_id) {
            return ArrayVec::new();
        }

        self.proposals().get(proposal_id).actions
    }

    fn require_valid_proposal_id(&self, proposal_id: ProposalId) {
        require!(
            self.is_valid_proposal_id(proposal_id),
            "Invalid proposal ID"
        );
    }

    fn is_valid_proposal_id(&self, proposal_id: ProposalId) -> bool {
        proposal_id >= 1 && proposal_id <= self.proposals().len()
    }

    fn proposal_exists(&self, proposal_id: ProposalId) -> bool {
        self.is_valid_proposal_id(proposal_id) && !self.proposals().item_is_empty(proposal_id)
    }
}
