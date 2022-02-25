#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod config;
mod deposit;
mod proposal;

#[elrond_wasm::contract]
pub trait Governance: config::Config {
    #[init]
    fn init(
        &self,
        quorum: BigUint,
        voting_delay_in_blocks: u64,
        voting_period_in_blocks: u64,
        max_actions_per_proposal: usize,
        governance_token_id: TokenIdentifier,
        min_token_balance_for_proposal: BigUint,
    ) {
        self.try_change_quorum(quorum);
        self.try_change_governance_token(governance_token_id);
        self.try_change_voting_delay_in_blocks(voting_delay_in_blocks);
        self.try_change_voting_period_in_blocks(voting_period_in_blocks);
        self.try_change_max_actions_per_proposal(max_actions_per_proposal);
        self.try_change_min_token_balance_for_proposing(min_token_balance_for_proposal);
    }

    #[payable("*")]
    #[endpoint]
    fn propose(&self) -> usize {
        0
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
