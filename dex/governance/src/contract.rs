#![no_std]
#![feature(generic_associated_types)]

use proposal::ProposalCreationArgs;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub mod config;
pub mod errors;
mod lib;
pub mod proposal;
mod validation;
pub mod vote;

use crate::errors::*;
use crate::proposal::*;
use crate::vote::*;

#[elrond_wasm::contract]
pub trait Governance:
    config::Config + validation::Validation + proposal::ProposalHelper + lib::Lib + vote::VoteHelper
{
    #[init]
    fn init(
        &self,
        quorum: BigUint,
        voting_delay_in_blocks: u64,
        voting_period_in_blocks: u64,
        vote_nft_id: TokenIdentifier,
        mex_token_id: TokenIdentifier,
        min_weight_for_proposal: BigUint,
        governance_token_ids: ManagedVec<TokenIdentifier>,
    ) {
        self.try_change_quorum(quorum);
        self.try_change_vote_nft_id(vote_nft_id);
        self.try_change_mex_token_id(mex_token_id);
        self.try_change_governance_token_ids(governance_token_ids);
        self.try_change_voting_delay_in_blocks(voting_delay_in_blocks);
        self.try_change_voting_period_in_blocks(voting_period_in_blocks);
        self.try_change_min_weight_for_proposal(min_weight_for_proposal);
    }

    #[payable("*")]
    #[endpoint]
    fn propose(&self, args: ProposalCreationArgs<Self::Api>) -> u64 {
        let payment = self.call_value().payment();
        self.require_is_accepted_payment_for_proposal(&payment);

        let vote_weight = self.get_vote_weight(&payment);
        let min_weight = self.min_weight_for_proposal().get();
        require!(vote_weight >= min_weight, NOT_ENOUGH_FUNDS_TO_PROPOSE);

        let mut proposal = self.new_proposal_from_args(args);
        self.proposal_id_counter().set(proposal.id + 1);

        proposal.num_upvotes = vote_weight.clone();
        self.proposal(proposal.id).set(&proposal);

        let vote_nft = self.create_vote_nft(proposal.id, VoteType::Upvote, vote_weight, payment);
        self.send_back(vote_nft);

        proposal.id
    }

    #[payable("*")]
    #[endpoint]
    fn upvote(&self, proposal_id: u64) {
        self.vote(proposal_id, VoteType::Upvote)
    }

    #[payable("*")]
    #[endpoint]
    fn downvote(&self, proposal_id: u64) {
        self.vote(proposal_id, VoteType::DownVote)
    }

    #[endpoint]
    fn execute(&self, proposal_id: u64) {
        require!(!self.proposal(proposal_id).is_empty(), PROPOSAL_NOT_FOUND);
        let mut proposal = self.proposal(proposal_id).get();

        let pstat = self.get_proposal_status(&proposal);
        require!(pstat == ProposalStatus::Succeeded, PROPOSAL_NOT_SUCCEEDED);

        self.execute_proposal(&proposal);
        proposal.was_executed = true;
        self.proposal(proposal_id).set(&proposal);
    }

    fn vote(&self, proposal_id: u64, vote_type: VoteType) {
        require!(!self.proposal(proposal_id).is_empty(), PROPOSAL_NOT_FOUND);
        let mut proposal = self.proposal(proposal_id).get();

        let pstat = self.get_proposal_status(&proposal);
        require!(pstat == ProposalStatus::Active, PROPOSAL_NOT_ACTIVE);

        let payment = self.call_value().payment();
        self.require_is_accepted_payment_for_voting(&payment);

        let vote_weight = self.get_vote_weight(&payment);
        match vote_type {
            VoteType::Upvote => proposal.num_upvotes += &vote_weight,
            VoteType::DownVote => proposal.num_downvotes += &vote_weight,
        }

        let vote_nft = self.create_vote_nft(proposal.id, vote_type, vote_weight, payment);
        self.send_back(vote_nft);

        self.proposal(proposal_id).set(&proposal);
    }

    #[payable("*")]
    #[endpoint]
    fn redeem(&self) {
        let payment = self.call_value().payment();

        let vote_nft_id = self.vote_nft_id().get();
        require!(payment.token_identifier == vote_nft_id, BAD_TOKEN_ID);

        let mut attr = self.get_vote_attr(&payment);
        require!(!attr.was_redeemed, ALREADY_REDEEMED);

        let proposal = self.proposal(attr.proposal_id).get();
        let pstat = self.get_proposal_status(&proposal);

        match pstat {
            ProposalStatus::Defeated | ProposalStatus::Executed => {
                attr.was_redeemed = true;
                self.update_vote_nft_attributes(&vote_nft_id, payment.token_nonce, &attr);

                self.send_back(attr.payment.clone());
                self.send_back(payment);
            }
            ProposalStatus::Succeeded => {
                sc_panic!(PROPOSAL_NEEDS_TO_BE_EXECUTED)
            }
            ProposalStatus::Active | ProposalStatus::Pending => {
                sc_panic!(VOTING_PERIOD_NOT_ENDED)
            }
        }
    }
}
