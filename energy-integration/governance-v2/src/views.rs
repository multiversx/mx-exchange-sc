multiversx_sc::imports!();

use crate::{
    proposal::{
        GovernanceAction, GovernanceProposalStatus, ProposalId, MAX_GOVERNANCE_PROPOSAL_ACTIONS,
    },
    FULL_PERCENTAGE,
};

use weekly_rewards_splitting::{events::Week, global_info::ProxyTrait as _};

#[multiversx_sc::module]
pub trait ViewsModule:
    crate::proposal_storage::ProposalStorageModule
    + crate::configurable::ConfigurablePropertiesModule
    + crate::caller_check::CallerCheckModule
    + permissions_module::PermissionsModule
    + energy_query::EnergyQueryModule
{
    #[view(getProposalStatus)]
    fn get_proposal_status(&self, proposal_id: ProposalId) -> GovernanceProposalStatus {
        if !self.proposal_exists(proposal_id) {
            return GovernanceProposalStatus::None;
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

        if self.quorum_reached(proposal_id) && self.vote_reached(proposal_id) {
            GovernanceProposalStatus::Succeeded
        } else if self.vote_down_with_veto(proposal_id) {
            GovernanceProposalStatus::DefeatedWithVeto
        } else {
            GovernanceProposalStatus::Defeated
        }
    }

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
        let fees_collector_addr = self.fees_collector_address().get();
        let last_global_update_week: Week = self
            .fees_collector_proxy(fees_collector_addr.clone())
            .last_global_update_week()
            .execute_on_dest_context();

        let total_energy: BigUint = self
            .fees_collector_proxy(fees_collector_addr)
            .total_energy_for_week(last_global_update_week)
            .execute_on_dest_context();

        let proposal = self.proposals().get(proposal_id);
        let required_minimum_percentage = proposal.minimum_quorum;

        let current_quorum = self.get_current_quorum(proposal_id);
        let current_quorum_percentage =
            current_quorum.clone() * FULL_PERCENTAGE / total_energy.clone();

        current_quorum_percentage > required_minimum_percentage
    }

    #[view(getQuorum)]
    fn get_current_quorum(&self, proposal_id: ProposalId) -> BigUint {
        if !self.proposal_exists(proposal_id) {
            sc_panic!("Proposal does not exist");
        }

        self.proposal_votes(proposal_id).get().quorum
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

    #[proxy]
    fn fees_collector_proxy(&self, sc_address: ManagedAddress) -> fees_collector::Proxy<Self::Api>;

    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;
}
