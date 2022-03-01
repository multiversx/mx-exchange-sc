elrond_wasm::imports!();

use crate::proposal::*;

#[elrond_wasm::module]
pub trait Config {
    #[endpoint(changeQuorum)]
    fn change_quorum(&self, new_value: BigUint) {
        self.require_caller_self();

        self.try_change_quorum(new_value);
    }

    #[endpoint(changeMinTokenBalanceForProposing)]
    fn change_min_token_balance_for_proposing(&self, new_value: BigUint) {
        self.require_caller_self();

        self.try_change_min_token_balance_for_proposing(new_value);
    }

    #[endpoint(changeMaxActionsPerProposal)]
    fn change_max_actions_per_proposal(&self, new_value: usize) {
        self.require_caller_self();

        self.try_change_max_actions_per_proposal(new_value);
    }

    #[endpoint(changeVotingDelayInBlocks)]
    fn change_voting_delay_in_blocks(&self, new_value: u64) {
        self.require_caller_self();

        self.try_change_voting_delay_in_blocks(new_value);
    }

    #[endpoint(changeVotingPeriodInBlocks)]
    fn change_voting_period_in_blocks(&self, new_value: u64) {
        self.require_caller_self();

        self.try_change_voting_period_in_blocks(new_value);
    }

    fn require_caller_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller == sc_address,
            "Only the SC itself may call this function"
        );
    }

    fn try_change_mex_token_id(&self, token_id: TokenIdentifier) {
        require!(
            token_id.is_esdt(),
            "Invalid ESDT token ID provided for vote_nft"
        );

        self.mex_token_id().set(&token_id);
    }

    fn try_change_vote_nft_id(&self, token_id: TokenIdentifier) {
        require!(
            token_id.is_esdt(),
            "Invalid ESDT token ID provided for vote_nft"
        );

        self.vote_nft_id().set(&token_id);
    }

    fn try_change_governance_token_ids(&self, token_ids: ManagedVec<TokenIdentifier>) {
        for token_id in token_ids.iter() {
            require!(
                token_id.is_esdt(),
                "Invalid ESDT token ID provided for token_ids"
            );
        }

        self.governance_token_ids().set(&token_ids);
    }

    fn try_change_quorum(&self, new_value: BigUint) {
        require!(new_value != 0, "Quorum can't be set to 0");

        self.quorum().set(&new_value);
    }

    fn try_change_min_token_balance_for_proposing(&self, new_value: BigUint) {
        require!(
            new_value != 0,
            "Min token balance for proposing can't be set to 0"
        );

        self.min_token_balance_for_proposing().set(&new_value);
    }

    fn try_change_max_actions_per_proposal(&self, new_value: usize) {
        require!(new_value != 0, "Max actions per proposal can't be set to 0");

        self.max_actions_per_proposal().set(&new_value);
    }

    fn try_change_voting_delay_in_blocks(&self, new_value: u64) {
        require!(new_value != 0, "Voting delay in blocks can't be set to 0");

        self.voting_delay_in_blocks().set(&new_value);
    }

    fn try_change_voting_period_in_blocks(&self, new_value: u64) {
        require!(
            new_value != 0,
            "Voting period (in blocks) can't be set to 0"
        );

        self.voting_period_in_blocks().set(&new_value);
    }

    #[view(getGovernanceTokenId)]
    #[storage_mapper("governanceTokenIds")]
    fn governance_token_ids(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;

    #[view(getQuorum)]
    #[storage_mapper("quorum")]
    fn quorum(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinTokenBalanceForProposing)]
    #[storage_mapper("minTokenBalanceForProposing")]
    fn min_token_balance_for_proposing(&self) -> SingleValueMapper<BigUint>;

    #[view(getMaxActionsPerProposal)]
    #[storage_mapper("maxActionsPerProposal")]
    fn max_actions_per_proposal(&self) -> SingleValueMapper<usize>;

    #[view(getVotingDelayInBlocks)]
    #[storage_mapper("votingDelayInBlocks")]
    fn voting_delay_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getVotingPeriodInBlocks)]
    #[storage_mapper("votingPeriodInBlocks")]
    fn voting_period_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getProposal)]
    #[storage_mapper("proposal")]
    fn proposal(&self, id: u64) -> SingleValueMapper<Proposal<Self::Api>>;

    #[view(getProposalIdCounter)]
    #[storage_mapper("proposalIdCounter")]
    fn proposal_id_counter(&self) -> SingleValueMapper<u64>;

    #[view(getVoteNFTId)]
    #[storage_mapper("voteNFTId")]
    fn vote_nft_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getMexTokenId)]
    #[storage_mapper("mexTokenId")]
    fn mex_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
