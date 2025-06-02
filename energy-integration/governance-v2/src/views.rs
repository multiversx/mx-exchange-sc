multiversx_sc::imports!();

use crate::{
    proposal::{GovernanceProposalStatus, ProposalId},
    FULL_PERCENTAGE,
};

#[multiversx_sc::module]
pub trait ViewsModule:
    crate::proposal_storage::ProposalStorageModule
    + crate::configurable::ConfigurablePropertiesModule
    + permissions_module::PermissionsModule
    + energy_query::EnergyQueryModule
{
    #[view(getProposalStatus)]
    fn get_proposal_status(&self, proposal_id: ProposalId) -> GovernanceProposalStatus {
        if !self.proposal_exists(proposal_id) {
            return GovernanceProposalStatus::None;
        }

        let current_timestamp = self.blockchain().get_block_timestamp();
        let proposal = self.proposals().get(proposal_id);
        let proposal_timestamp = proposal.proposal_start_timestamp;

        let voting_delay = proposal.voting_delay_in_seconds;
        let voting_period = proposal.voting_period_in_seconds;

        let voting_start = proposal_timestamp + voting_delay;
        let voting_end = voting_start + voting_period;

        if current_timestamp < voting_start {
            return GovernanceProposalStatus::Pending;
        }
        if current_timestamp >= voting_start && current_timestamp < voting_end {
            return GovernanceProposalStatus::Active;
        }

        if self.quorum_reached(proposal_id) && self.vote_reached(proposal_id) {
            GovernanceProposalStatus::Succeeded
        } else if self.vote_down_with_veto(proposal_id) {
            GovernanceProposalStatus::DefeatedWithVeto
        } else {
            GovernanceProposalStatus::Defeated
        }
    }

    // private

    fn vote_reached(&self, proposal_id: ProposalId) -> bool {
        let proposal_votes = self.proposal_votes(proposal_id).get();
        let total_votes = proposal_votes.get_total_votes();
        let total_up_votes = proposal_votes.up_votes;
        let total_down_veto_votes = proposal_votes.down_veto_votes;
        let third_total_votes = &total_votes / 3u64;
        let half_total_votes = &total_votes / 2u64;

        if total_down_veto_votes > third_total_votes {
            false
        } else {
            total_up_votes > half_total_votes
        }
    }

    fn vote_down_with_veto(&self, proposal_id: ProposalId) -> bool {
        let proposal_votes = self.proposal_votes(proposal_id).get();
        let total_votes = proposal_votes.get_total_votes();
        let total_down_veto_votes = proposal_votes.down_veto_votes;
        let third_total_votes = &total_votes / 3u64;

        total_down_veto_votes > third_total_votes
    }

    fn quorum_reached(&self, proposal_id: ProposalId) -> bool {
        let proposal = self.proposals().get(proposal_id);
        let total_quorum_for_proposal = proposal.total_quorum;
        let required_minimum_percentage = BigUint::from(proposal.minimum_quorum);
        let current_quorum = self.proposal_votes(proposal_id).get().quorum;

        current_quorum * FULL_PERCENTAGE >= required_minimum_percentage * total_quorum_for_proposal
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

    #[only_owner]
    #[endpoint(changeFeesCollectorAddress)]
    fn change_fees_collector_address(&self, new_value: ManagedAddress) {
        self.fees_collector_address().set(new_value);
    }

    #[proxy]
    fn fees_collector_proxy(&self, sc_address: ManagedAddress) -> fees_collector::Proxy<Self::Api>;

    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;
}
