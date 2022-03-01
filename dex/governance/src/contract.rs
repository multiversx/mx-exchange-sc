#![no_std]
#![feature(generic_associated_types)]

use proposal::ProposalCreationArgs;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod config;
mod lib;
mod proposal;
mod validation;
mod vote;

use vote::*;

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
        max_actions_per_proposal: usize,
        min_token_balance_for_proposal: BigUint,
        governance_token_ids: ManagedVec<TokenIdentifier>,
    ) {
        self.try_change_quorum(quorum);
        self.try_change_vote_nft_id(vote_nft_id);
        self.try_change_mex_token_id(mex_token_id);
        self.try_change_governance_token_ids(governance_token_ids);
        self.try_change_voting_delay_in_blocks(voting_delay_in_blocks);
        self.try_change_voting_period_in_blocks(voting_period_in_blocks);
        self.try_change_max_actions_per_proposal(max_actions_per_proposal);
        self.try_change_min_token_balance_for_proposing(min_token_balance_for_proposal);
    }

    #[payable("*")]
    #[endpoint]
    fn propose(&self, args: ProposalCreationArgs<Self::Api>) -> u64 {
        let payment = self.call_value().payment();
        self.require_is_accepted_payment_for_proposal(&payment);
        self.require_are_accepted_args_for_proposal(&args);

        let mut proposal = self.new_proposal_from_args(args);
        self.proposal_id_counter().set(proposal.id + 1);

        let vote_weight = self.get_vote_weight(&payment);
        proposal.num_upvotes = vote_weight.clone();

        let vote_nft = self.create_vote_nft(proposal.id, VoteType::Upvote, vote_weight, payment);
        self.send_back(vote_nft);

        proposal.id
    }

    #[payable("*")]
    #[endpoint]
    fn vote(&self, _proposal_id: usize) {}

    #[payable("*")]
    #[endpoint]
    fn downvote(&self, _proposal_id: usize) {}

    #[endpoint]
    fn execute(&self, _proposal_id: usize) {}
}
