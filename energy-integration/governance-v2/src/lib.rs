#![no_std]

multiversx_sc::imports!();

pub mod caller_check;
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
const FULL_PERCENTAGE: u64 = 10_000;

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait GovernanceV2:
    configurable::ConfigurablePropertiesModule
    + events::EventsModule
    + proposal_storage::ProposalStorageModule
    + caller_check::CallerCheckModule
    + views::ViewsModule
{
    /// - `min_energy_for_propose` - the minimum energy required for submitting a proposal;
    /// - `min_fee_for_propose` - the minimum fee required for submitting a proposal;
    /// - `quorum` - the minimum number of (`votes` minus `downvotes`) at the end of voting period;
    /// - `votingDelayInBlocks` - Number of blocks to wait after a block is proposed before being able to vote/downvote that proposal;
    /// - `votingPeriodInBlocks` - Number of blocks the voting period lasts (voting delay does not count towards this);
    /// - `withdraw_percentage_defeated` - The percentage used to return in case of DownVetoVote;
    /// - `fee_token` - The token used to pay the fee for governance proposal;
    #[init]
    fn init(
        &self,
        // min_energy_for_propose: BigUint,
        min_fee_for_propose: BigUint,
        quorum_percentage: BigUint,
        voting_delay_in_blocks: u64,
        voting_period_in_blocks: u64,
        withdraw_percentage_defeated: u64,
        fee_token: TokenIdentifier,
    ) {
        // self.try_change_min_energy_for_propose(min_energy_for_propose);
        self.try_change_min_fee_for_propose(min_fee_for_propose);
        self.try_change_quorum_percentage(quorum_percentage);
        self.try_change_voting_delay_in_blocks(voting_delay_in_blocks);
        self.try_change_voting_period_in_blocks(voting_period_in_blocks);
        self.try_change_withdraw_percentage_defeated(withdraw_percentage_defeated);
        self.try_change_fee_token_id(fee_token);
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
    #[payable("*")]
    #[endpoint]
    fn propose(
        &self,
        root_hash: ManagedByteArray<HASH_LENGTH>,
        total_quorum: BigUint<Self::Api>,
        description: ManagedBuffer,
        actions: MultiValueEncoded<GovernanceActionAsMultiArg<Self::Api>>,
    ) -> ProposalId {
        self.require_caller_not_self();
        require!(!root_hash.is_empty(), INVALID_ROOT_HASH);
        require!(!actions.is_empty(), PROPOSAL_NO_ACTION);
        require!(
            actions.len() <= MAX_GOVERNANCE_PROPOSAL_ACTIONS,
            EXEEDED_MAX_ACTIONS
        );

        let proposer = self.blockchain().get_caller();
        // let user_energy = self.get_energy_amount_non_zero(&proposer);
        // let min_energy_for_propose = self.min_energy_for_propose().get();
        // require!(user_energy >= min_energy_for_propose, NOT_ENOUGH_ENERGY);

        let user_fee = self.call_value().single_esdt();
        require!(
            self.fee_token_id().get() == user_fee.token_identifier,
            WRONG_TOKEN_ID
        );
        require!(
            self.min_fee_for_propose().get() == user_fee.amount,
            NOT_ENOUGH_FEE
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
            TOO_MUCH_GAS
        );

        let minimum_quorum = self.quorum_percentage().get();
        let voting_delay_in_blocks = self.voting_delay_in_blocks().get();
        let voting_period_in_blocks = self.voting_period_in_blocks().get();
        let withdraw_percentage_defeated = self.withdraw_percentage_defeated().get();
        let current_block = self.blockchain().get_block_nonce();

        let proposal = GovernanceProposal {
            proposal_id: self.proposals().len() + 1,
            proposer: proposer.clone(),
            description,
            root_hash,
            actions: gov_actions,
            fee_payment: user_fee,
            minimum_quorum,
            voting_delay_in_blocks,
            voting_period_in_blocks,
            withdraw_percentage_defeated,
            total_quorum,
            proposal_start_block: current_block,
        };
        let proposal_id = self.proposals().push(&proposal);

        self.proposal_votes(proposal_id)
            .set(ProposalVotes::default());
        self.proposal_created_event(proposal_id, &proposer, current_block, &proposal);

        proposal_id
    }

    /// Vote on a proposal. The voting power depends on the user's quorum (number of tokens).
    #[endpoint]
    fn vote(
        &self,
        proposal_id: ProposalId,
        vote: VoteType,
        user_quorum: BigUint<Self::Api>,
        proof: ArrayVec<ManagedByteArray<HASH_LENGTH>, PROOF_LENGTH>,
    ) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::Active,
            PROPOSAL_NOT_ACTIVE
        );

        let voter = self.blockchain().get_caller();
        let new_user = self.user_voted_proposals(&voter).insert(proposal_id);
        require!(new_user, ALREADY_VOTED_ERR_MSG);

        let voting_power = user_quorum.sqrt();

        match self.get_root_hash(proposal_id) {
            OptionalValue::None => {
                sc_panic!(NO_PROPOSAL);
            }
            OptionalValue::Some(root_hash) => {
                require!(
                    self.verify_merkle_proof(voting_power.clone(), proof, root_hash),
                    INVALID_MERKLE_PROOF
                );
            }
        }

        match vote {
            VoteType::UpVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.up_votes += &voting_power.clone();
                    proposal_votes.quorum += &voting_power.clone();
                });
                self.up_vote_cast_event(&voter, proposal_id, &voting_power, &user_quorum);
            }
            VoteType::DownVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.down_votes += &voting_power.clone();
                    proposal_votes.quorum += &voting_power.clone();
                });
                self.down_vote_cast_event(&voter, proposal_id, &voting_power, &user_quorum);
            }
            VoteType::DownVetoVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.down_veto_votes += &voting_power.clone();
                    proposal_votes.quorum += &voting_power.clone();
                });
                self.down_veto_vote_cast_event(&voter, proposal_id, &voting_power, &user_quorum);
            }
            VoteType::AbstainVote => {
                self.proposal_votes(proposal_id).update(|proposal_votes| {
                    proposal_votes.abstain_votes += &voting_power.clone();
                    proposal_votes.quorum += &voting_power.clone();
                });
                self.abstain_vote_cast_event(&voter, proposal_id, &voting_power, &user_quorum);
            }
        }
        self.user_voted_proposals(&voter).insert(proposal_id);
    }

    /// Cancel a proposed action. This can be done:
    /// - by the proposer, at any time
    /// - by anyone, if the proposal was defeated
    #[endpoint]
    fn cancel(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();

        match self.get_proposal_status(proposal_id) {
            GovernanceProposalStatus::None => {
                sc_panic!(NO_PROPOSAL);
            }
            GovernanceProposalStatus::Pending => {
                let proposal = self.proposals().get(proposal_id);
                let caller = self.blockchain().get_caller();

                require!(caller == proposal.proposer, ONLY_PROPOSER_CANCEL);
                self.refund_proposal_fee(proposal_id, &proposal.fee_payment.amount);
                self.clear_proposal(proposal_id);
                self.proposal_canceled_event(proposal_id);
            }
            _ => {
                sc_panic!("Action may not be cancelled");
            }
        }
    }

    /// When a proposal was defeated, the proposer can withdraw
    /// a part of the FEE.
    #[endpoint(withdrawDeposit)]
    fn withdraw_deposit(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        let caller = self.blockchain().get_caller();

        match self.get_proposal_status(proposal_id) {
            GovernanceProposalStatus::None => {
                sc_panic!(NO_PROPOSAL);
            }
            GovernanceProposalStatus::Succeeded | GovernanceProposalStatus::Defeated => {
                let proposal = self.proposals().get(proposal_id);

                require!(caller == proposal.proposer, ONLY_PROPOSER_WITHDRAW);

                self.refund_proposal_fee(proposal_id, &proposal.fee_payment.amount);
            }
            GovernanceProposalStatus::DefeatedWithVeto => {
                let proposal = self.proposals().get(proposal_id);
                let refund_percentage = BigUint::from(proposal.withdraw_percentage_defeated);
                let refund_amount =
                    refund_percentage * proposal.fee_payment.amount.clone() / FULL_PERCENTAGE;

                require!(caller == proposal.proposer, ONLY_PROPOSER_WITHDRAW);

                self.refund_proposal_fee(proposal_id, &refund_amount);
                let remaining_fee = proposal.fee_payment.amount - refund_amount;

                self.proposal_remaining_fees().update(|fees| {
                    fees.push(EsdtTokenPayment::new(
                        proposal.fee_payment.token_identifier,
                        proposal.fee_payment.token_nonce,
                        remaining_fee,
                    ));
                });
            }
            _ => {
                sc_panic!(WITHDRAW_NOT_ALLOWED);
            }
        }
        self.proposal_withdraw_after_defeated_event(proposal_id);
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

    fn refund_proposal_fee(&self, proposal_id: ProposalId, refund_amount: &BigUint) {
        let proposal: GovernanceProposal<<Self as ContractBase>::Api> =
            self.proposals().get(proposal_id);

        self.send().direct_esdt(
            &proposal.proposer,
            &proposal.fee_payment.token_identifier,
            proposal.fee_payment.token_nonce,
            refund_amount,
        );
    }

    fn verify_merkle_proof(
        &self,
        power: BigUint<Self::Api>,
        proof: ArrayVec<ManagedByteArray<HASH_LENGTH>, PROOF_LENGTH>,
        root_hash: ManagedByteArray<HASH_LENGTH>,
    ) -> bool {
        let caller = self.blockchain().get_caller();
        let mut leaf_bytes = caller.as_managed_buffer().clone();

        let p = power.to_bytes_be_buffer();
        leaf_bytes.append(&p);

        let mut hash = self.crypto().sha256(&leaf_bytes);
        for proof_item in proof {
            if BigUint::from(hash.as_managed_buffer())
                < BigUint::from(proof_item.as_managed_buffer())
            {
                let mut tst = hash.as_managed_buffer().clone();
                tst.append(proof_item.as_managed_buffer());

                hash = self.crypto().sha256(tst);
            } else {
                let mut tst = proof_item.as_managed_buffer().clone();
                tst.append(hash.as_managed_buffer());

                hash = self.crypto().sha256(tst);
            }
        }

        hash == root_hash
    }

    #[storage_mapper("proposalRemainingFees")]
    fn proposal_remaining_fees(&self)
        -> SingleValueMapper<ManagedVec<EsdtTokenPayment<Self::Api>>>;
}
