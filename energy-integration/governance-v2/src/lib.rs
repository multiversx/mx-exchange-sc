#![no_std]

elrond_wasm::imports!();

pub mod configurable;
pub mod events;
pub mod proposal;
pub mod proposal_storage;
pub mod views;

use proposal::*;

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
    /// Used to deposit tokens for "payable" actions.
    /// Funds will be returned if the proposal is defeated.
    /// To keep the logic simple, all tokens have to be deposited at once
    #[payable("*")]
    #[endpoint(depositTokensForProposal)]
    fn deposit_tokens_for_proposal(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);

        let depositor_mapper = self.payments_depositor(proposal_id);
        require!(depositor_mapper.is_empty(), "Payments already deposited");

        let required_payments = self.required_payments_for_proposal(proposal_id).get();
        require!(
            !required_payments.is_empty(),
            "This proposal requires no payments"
        );

        let actual_payments = self.call_value().all_esdt_transfers();
        require!(
            actual_payments == required_payments,
            "Invalid payments, must match the required payments"
        );

        let caller = self.blockchain().get_caller();
        depositor_mapper.set(&caller);

        self.user_deposit_event(&caller, proposal_id, &actual_payments);
    }

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
    fn propose(
        &self,
        description: ManagedBuffer,
        actions: MultiValueEncoded<GovernanceActionAsMultiArg<Self::Api>>,
    ) -> ProposalId {
        self.require_caller_not_self();
        require!(!actions.is_empty(), "Proposal has no actions");
        require!(
            actions.len() <= MAX_GOVERNANCE_PROPOSAL_ACTIONS,
            "Exceeded max actions per proposal"
        );

        let proposer = self.blockchain().get_caller();
        let user_energy = self.get_energy_amount_non_zero(&proposer);
        let min_energy_for_propose = self.min_energy_for_propose().get();
        require!(
            user_energy >= min_energy_for_propose,
            "Not enough energy for propose"
        );

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
            proposer: proposer.clone(),
            description,
            actions: gov_actions,
        };
        let proposal_id = self.proposals().push(&proposal);

        if !payments_for_action.is_empty() {
            self.required_payments_for_proposal(proposal_id)
                .set(&payments_for_action);
        }
        
        let proposal_votes = ProposalVotes {
            up_votes: user_energy,
            down_votes: BigUint::zero(),
            down_votes_veto: BigUint::zero(),
            abstain: BigUint::zero(),
        };

        self.proposal_votes(proposal_id).set(proposal_votes);
        let _ = self.user_voted_proposals(&proposer).insert(proposal_id);

        let current_block = self.blockchain().get_block_nonce();
        self.proposal_start_block(proposal_id).set(current_block);

        self.proposal_created_event(proposal_id, &proposer, current_block, &proposal);

        proposal_id
    }

    /// Vote on a proposal. The voting power depends on the user's energy.
    #[endpoint]
    fn vote(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Active,
            "Proposal is not active"
        );

        let voter = self.blockchain().get_caller();
        let new_user = self.user_voted_proposals(&voter).insert(proposal_id);
        require!(new_user, ALREADY_VOTED_ERR_MSG);

        let user_energy = self.get_energy_amount_non_zero(&voter);
        self.proposal_votes(proposal_id).update(|proposal_votes| {
            proposal_votes.up_votes += user_energy.clone();
        });

        self.vote_cast_event(&voter, proposal_id, &user_energy);
    }

    /// Downvote a proposal. The voting power depends on the user's energy.
    #[endpoint]
    fn downvote(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Active,
            "Proposal is not active"
        );

        let downvoter = self.blockchain().get_caller();
        let new_user = self.user_voted_proposals(&downvoter).insert(proposal_id);
        require!(new_user, ALREADY_VOTED_ERR_MSG);

        let user_energy = self.get_energy_amount_non_zero(&downvoter);
        self.proposal_votes(proposal_id).update(|proposal_votes| {
            proposal_votes.down_votes += user_energy.clone();
        });
        self.downvote_cast_event(&downvoter, proposal_id, &user_energy);
    }

    /// Queue a proposal for execution.
    /// This can be done only if the proposal has reached the quorum.
    /// A proposal is considered successful and ready for queing if
    /// total_votes + total_downvotes >= quorum && total_votes > total_downvotes
    /// i.e. at least 50% + 1 of the people voted "yes".
    ///
    /// Additionally, all the required payments must be deposited before calling this endpoint.
    #[endpoint]
    fn queue(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Succeeded,
            "Can only queue succeeded proposals"
        );
        require!(
            self.required_payments_for_proposal(proposal_id).is_empty()
                || !self.payments_depositor(proposal_id).is_empty(),
            "Payments for proposal not deposited"
        );

        let current_block = self.blockchain().get_block_nonce();
        self.proposal_queue_block(proposal_id).set(current_block);

        self.proposal_queued_event(proposal_id, current_block);
    }

    /// Execute a previously queued proposal.
    /// This will also clear the proposal from storage.
    #[endpoint]
    fn execute(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Queued,
            "Can only execute queued proposals"
        );

        let current_block = self.blockchain().get_block_nonce();
        let lock_blocks = self.lock_time_after_voting_ends_in_blocks().get();

        let lock_start = self.proposal_queue_block(proposal_id).get();
        let lock_end = lock_start + lock_blocks;

        require!(
            current_block >= lock_end,
            "Proposal is in timelock status. Try again later"
        );

        let proposal = self.proposals().get(proposal_id);
        let total_gas_needed = self.total_gas_needed(&proposal.actions);
        let gas_left = self.blockchain().get_gas_left();

        require!(
            gas_left > total_gas_needed,
            "Not enough gas to execute all proposals"
        );

        self.clear_proposal(proposal_id);

        for action in proposal.actions {
            let mut contract_call = self
                .send()
                .contract_call::<()>(action.dest_address, action.function_name)
                .with_gas_limit(action.gas_limit);

            if !action.payments.is_empty() {
                contract_call = contract_call.with_multi_token_transfer(action.payments);
            }

            for arg in &action.arguments {
                contract_call.push_arg_managed_buffer(arg);
            }

            contract_call.transfer_execute();
        }

        self.proposal_executed_event(proposal_id);
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
}
