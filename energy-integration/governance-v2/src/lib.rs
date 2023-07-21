#![no_std]

elrond_wasm::imports!();

pub mod configurable;
pub mod events;
pub mod proposal;
pub mod proposal_storage;
pub mod views;

use proposal::*;
use proposal_storage::VoteType;

use crate::proposal_storage::ProposalVotes;

const MAX_GAS_LIMIT_PER_BLOCK: u64 = 600_000_000;
static ALREADY_VOTED_ERR_MSG: &[u8] = b"Already voted for this proposal";

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[elrond_wasm::contract]
pub trait GovernanceV2:
    configurable::ConfigurablePropertiesModule
    + events::EventsModule
    + proposal_storage::ProposalStorageModule
    + views::ViewsModule
    + energy_query::EnergyQueryModule
{
    /// Propose a list of actions.
    /// A maximum of MAX_GOVERNANCE_PROPOSAL_ACTIONS can be proposed at a time.
    ///
    /// An action has the following format:
    ///     - gas limit for action execution
    ///     - destination address
    ///     - a vector of ESDT transfers, in the form of ManagedVec<EsdTokenPayment>
    ///     - endpoint to be called on the destination
    ///     - a vector of arguments for the endpoint, in the form of ManagedVec<ManagedBuffer>
    ///
    /// The proposer's energy is automatically used for voting already.
    ///
    /// Returns the ID of the newly created proposal.
    #[endpoint]
    #[only_owner]
    fn propose(
        &self,
        root_hash: ManagedByteArray<HASH_LENGTH>,
        description: ManagedBuffer,
        actions: MultiValueEncoded<GovernanceActionAsMultiArg<Self::Api>>,
    ) -> ProposalId {
        self.require_caller_not_self();
        require!(!root_hash.is_empty(), "Invalid root hash provided");
        require!(
            actions.len() <= MAX_GOVERNANCE_PROPOSAL_ACTIONS,
            "Exceeded max actions per proposal"
        );

        let proposer = self.blockchain().get_caller();

        let mut gov_actions = ArrayVec::new();
        let mut payments_for_action = ManagedVec::new();
        for action_multiarg in actions {
            let gov_action = GovernanceAction::from(action_multiarg);
            require!(
                gov_action.gas_limit < MAX_GAS_LIMIT_PER_BLOCK,
                "A single action cannot use more than the max gas limit per block"
            );

            if !gov_action.payments.is_empty() {
                payments_for_action.append_vec(gov_action.payments.clone());
            }

            unsafe {
                gov_actions.push_unchecked(gov_action);
            }
        }

        require!(
            self.total_gas_needed(&gov_actions) < MAX_GAS_LIMIT_PER_BLOCK,
            "Actions require too much gas to be executed"
        );

        let proposal = GovernanceProposal {
            root_hash,
            proposer: proposer.clone(),
            description,
            actions: gov_actions,
        };
        let proposal_id = self.proposals().push(&proposal);

        if !payments_for_action.is_empty() {
            self.required_payments_for_proposal(proposal_id)
                .set(&payments_for_action);
        }

        let proposal_votes = ProposalVotes::new(
            BigUint::zero(),
            BigUint::zero(),
            BigUint::zero(),
            BigUint::zero(),
        );

        self.proposal_votes(proposal_id).set(proposal_votes);

        let current_block = self.blockchain().get_block_nonce();
        self.proposal_start_block(proposal_id).set(current_block);

        self.proposal_created_event(proposal_id, &proposer, current_block, &proposal);

        proposal_id
    }

    /// Vote on a proposal. The voting power depends on the user's energy.
    #[endpoint]
    fn vote(&self, proposal_id: ProposalId, vote: VoteType, power: BigUint<Self::Api>, proof: ArrayVec<ManagedByteArray<HASH_LENGTH>, PROOF_LENGTH>) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Active,
            "Proposal is not active"
        );

        let voter = self.blockchain().get_caller();
        let new_user = self.user_voted_proposals(&voter).insert(proposal_id);
        require!(new_user, ALREADY_VOTED_ERR_MSG);

        let user_energy = power.clone();
        match self.get_root_hash(proposal_id) {
            OptionalValue::None => {
                sc_panic!("Proposal does not exist");
            }
            OptionalValue::Some(root_hash) => {
                require!(self.verify_merkle_proof(power, proof, root_hash), "Invalid merkle proof provided");
            }
        }
        

        match vote {
            VoteType::UpVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.up_votes += &user_energy.clone();
                });
                self.up_vote_cast_event(&voter, proposal_id, &user_energy);
            }
            VoteType::DownVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.down_votes += &user_energy.clone();
                });
                self.down_vote_cast_event(&voter, proposal_id, &user_energy);
            }
            VoteType::DownVetoVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.down_veto_votes += &user_energy.clone();
                });
                self.down_veto_vote_cast_event(&voter, proposal_id, &user_energy);
            },
            VoteType::AbstainVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.abstain_votes += &user_energy.clone();
                });
                self.abstain_vote_cast_event(&voter, proposal_id, &user_energy);
        
            }
        }
        
        let _ = self.user_voted_proposals(&voter).insert(proposal_id);
    }

    /// Cancel a proposed action. This can be done:
    /// - by the proposer, at any time
    /// - by anyone, if the proposal was defeated
    #[endpoint]
    fn cancel(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();

        match self.get_proposal_status(proposal_id) {
            GovernanceProposalStatus::None => {
                sc_panic!("Proposal does not exist");
            }
            GovernanceProposalStatus::Pending => {
                let proposal = self.proposals().get(proposal_id);
                let caller = self.blockchain().get_caller();

                require!(
                    caller == proposal.proposer,
                    "Only original proposer may cancel a pending proposal"
                );
            }
            GovernanceProposalStatus::Defeated => {}
            _ => {
                sc_panic!("Action may not be cancelled");
            }
        }

        self.refund_payments(proposal_id);
        self.clear_proposal(proposal_id);

        self.proposal_canceled_event(proposal_id);
    }

    fn require_caller_not_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller != sc_address,
            "Cannot call this endpoint through proposed action"
        );
    }

    fn total_gas_needed(
        &self,
        actions: &ArrayVec<GovernanceAction<Self::Api>, MAX_GOVERNANCE_PROPOSAL_ACTIONS>,
    ) -> u64 {
        let mut total = 0;
        for action in actions {
            total += action.gas_limit;
        }

        total
    }

    fn refund_payments(&self, proposal_id: ProposalId) {
        let payments = self.required_payments_for_proposal(proposal_id).get();
        if payments.is_empty() {
            return;
        }

        let depositor_mapper = self.payments_depositor(proposal_id);
        if !depositor_mapper.is_empty() {
            let depositor = depositor_mapper.get();
            self.send().direct_multi(&depositor, &payments);
        }
    }

    fn clear_proposal(&self, proposal_id: ProposalId) {
        self.proposals().clear_entry(proposal_id);
        self.proposal_start_block(proposal_id).clear();
        self.proposal_queue_block(proposal_id).clear();

        self.required_payments_for_proposal(proposal_id).clear();
        self.payments_depositor(proposal_id).clear();

        self.proposal_votes(proposal_id).clear();
    }

    fn verify_merkle_proof(&self, power: BigUint<Self::Api>, proof: ArrayVec<ManagedByteArray<HASH_LENGTH>, PROOF_LENGTH>, root_hash: ManagedByteArray<HASH_LENGTH>) -> bool {
        let caller = self.blockchain().get_caller().clone();
        let mut leaf_bytes = caller.as_managed_buffer().clone();

        let p = power.to_bytes_be_buffer();
        leaf_bytes.append(&p);

        let mut hash = self.crypto().sha256(&leaf_bytes);
        for proof_item in proof {
            if BigUint::from(hash.as_managed_buffer()) < BigUint::from(proof_item.as_managed_buffer()) {
                let mut tst = hash.as_managed_buffer().clone();
                tst.append(proof_item.as_managed_buffer());

                hash = self.crypto().sha256(tst);
            } else {
                let mut tst = proof_item.as_managed_buffer().clone();
                tst.append(hash.as_managed_buffer());

                hash = self.crypto().sha256(tst);
            }
        }

        return hash == root_hash;
    }
}
