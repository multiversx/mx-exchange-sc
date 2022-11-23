#![no_std]

elrond_wasm::imports!();

pub mod configurable;
mod errors;
pub mod events;
pub mod proposal;
pub mod proposal_storage;
pub mod views;

use proposal::*;
use proposal_storage::VoteType;

use crate::errors::*;
use crate::proposal_storage::ProposalVotes;

const MAX_GAS_LIMIT_PER_BLOCK: u64 = 600_000_000;
const MIN_AMOUNT_PER_DEPOSIT: u64 = 1;
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
    /// Used to deposit tokens to gather threshold min_fee.
    /// Funds will be returned if the proposal is canceled.
    #[payable("*")]
    #[endpoint(depositTokensForProposal)]
    fn deposit_tokens_for_proposal(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);

        require!(
            !self.proposal_reached_min_fees(proposal_id),
            MIN_FEES_REACHED
        );

        let additional_fee = self.call_value().single_esdt();
        require!(
            self.fee_token_id().get() == additional_fee.token_identifier,
            WRONG_TOKEN_ID
        );
        require!(
            additional_fee.amount >= MIN_AMOUNT_PER_DEPOSIT,
            MIN_AMOUNT_NOT_REACHED
        );

        let caller = self.blockchain().get_caller();
        let mut proposal = self.proposals().get(proposal_id);
        proposal.fees.entries.push(FeeEntry {
            depositor_addr: caller.clone(),
            tokens: additional_fee.clone(),
        });
        proposal.fees.total_amount += additional_fee.amount.clone();

        self.proposals().set(proposal_id, &proposal);
        self.user_deposit_event(&caller, proposal_id, &additional_fee);
    }

    /// Used to claim deposited tokens to gather threshold min_fee.
    #[payable("*")]
    #[endpoint(claimDepositedTokens)]
    fn claim_deposited_tokens(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);

        require!(
            !self.proposal_reached_min_fees(proposal_id),
            MIN_FEES_REACHED
        );
        let caller = self.blockchain().get_caller();
        let mut proposal = self.proposals().get(proposal_id);

        let mut fees_to_send = ManagedVec::<Self::Api, FeeEntry<Self::Api>>::new();
        let mut i = 0;
        while i < proposal.fees.entries.len() {
            if proposal.fees.entries.get(i).depositor_addr == caller {
                fees_to_send.push(proposal.fees.entries.get(i));
                proposal.fees.entries.remove(i);
            } else {
                i += 1;
            }
        }

        for fee_entry in fees_to_send.iter() {
            let payment = fee_entry.tokens;

            self.send().direct_esdt(
                &fee_entry.depositor_addr,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }
    }

    /// Propose a list of actions.
    /// A maximum of MAX_GOVERNANCE_PROPOSAL_ACTIONS can be proposed at a time.
    ///
    /// An action has the following format:
    ///     - gas limit for action execution
    ///     - destination address
    ///     - a fee payment for proposal (if smaller than min_fee_for_propose, state: WaitForFee)
    ///     - endpoint to be called on the destination
    ///     - a vector of arguments for the endpoint, in the form of ManagedVec<ManagedBuffer>
    ///
    /// The proposer's energy is NOT automatically used for voting. A separate vote is needed.
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

        let user_fee = self.call_value().single_esdt();
        require!(
            self.fee_token_id().get() == user_fee.token_identifier,
            WRONG_TOKEN_ID
        );

        let mut gov_actions = ArrayVec::new();
        for action_multiarg in actions {
            let gov_action = GovernanceAction::from(action_multiarg);
            require!(
                gov_action.gas_limit < MAX_GAS_LIMIT_PER_BLOCK,
                "A single action cannot use more than the max gas limit per block"
            );

            unsafe {
                gov_actions.push_unchecked(gov_action);
            }
        }

        require!(
            self.total_gas_needed(&gov_actions) < MAX_GAS_LIMIT_PER_BLOCK,
            "Actions require too much gas to be executed"
        );

        let fees_entries = ManagedVec::from_single_item(FeeEntry {
            depositor_addr: proposer.clone(),
            tokens: user_fee.clone(),
        });

        let proposal = GovernanceProposal {
            proposer: proposer.clone(),
            description,
            actions: gov_actions,
            fees: ProposalFees {
                total_amount: user_fee.amount,
                entries: fees_entries,
            },
        };
        let proposal_id = self.proposals().push(&proposal);

        let proposal_votes = ProposalVotes::new();

        self.proposal_votes(proposal_id).set(proposal_votes);

        let current_block = self.blockchain().get_block_nonce();
        self.proposal_start_block(proposal_id).set(current_block);

        self.proposal_created_event(proposal_id, &proposer, current_block, &proposal);

        proposal_id
    }

    /// Vote on a proposal. The voting power depends on the user's energy.
    #[endpoint]
    fn vote(&self, proposal_id: ProposalId, vote: VoteType) {
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
            }
            VoteType::AbstainVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.abstain_votes += &user_energy.clone();
                });
                self.abstain_vote_cast_event(&voter, proposal_id, &user_energy);
            }
        }
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
            GovernanceProposalStatus::WaitingForFees => {
                self.refund_payments(proposal_id);
            }
            _ => {
                sc_panic!("Action may not be cancelled");
            }
        }

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
        let payments = self.proposals().get(proposal_id).fees;

        for fee_entry in payments.entries.iter() {
            let payment = fee_entry.tokens;
            self.send().direct_esdt(
                &fee_entry.depositor_addr,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }
    }

    fn clear_proposal(&self, proposal_id: ProposalId) {
        self.proposals().clear_entry(proposal_id);
        self.proposal_start_block(proposal_id).clear();
        self.proposal_queue_block(proposal_id).clear();
        self.proposal_votes(proposal_id).clear();
    }
}
