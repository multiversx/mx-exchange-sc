multiversx_sc::imports!();

use crate::errors::*;
use crate::proposal::*;

#[multiversx_sc::module]
pub trait Config {
    #[endpoint(changeQuorum)]
    fn change_quorum(&self, new_value: BigUint) {
        self.require_caller_self();

        self.try_change_quorum(new_value);
    }

    #[endpoint(changeMinTokenBalanceForProposing)]
    fn change_min_weight_for_proposal(&self, new_value: BigUint) {
        self.require_caller_self();

        self.try_change_min_weight_for_proposal(new_value);
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

    #[endpoint(changeGovernanceTokenIds)]
    fn change_governance_token_ids(&self, token_ids: ManagedVec<TokenIdentifier>) {
        self.require_caller_self();

        self.try_change_governance_token_ids(token_ids);
    }

    #[endpoint(changePriceProviders)]
    fn change_price_providers(
        &self,
        price_providers: MultiValueEncoded<MultiValue2<TokenIdentifier, ManagedAddress>>,
    ) {
        self.require_caller_self();

        self.try_change_price_providers(price_providers);
    }

    fn require_caller_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(caller == sc_address, INVALID_CALLER_NOT_SELF);
    }

    fn try_change_mex_token_id(&self, token_id: TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), INVALID_ESDT);

        self.mex_token_id().set(&token_id);
    }

    fn try_change_vote_nft_id(&self, token_id: TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), INVALID_ESDT);

        self.vote_nft_id().set(&token_id);
    }

    fn try_change_governance_token_ids(&self, token_ids: ManagedVec<TokenIdentifier>) {
        self.governance_token_ids().clear();

        for token_id in token_ids.into_iter() {
            require!(token_id.is_valid_esdt_identifier(), INVALID_ESDT);

            self.governance_token_ids().insert(token_id);
        }
    }

    fn try_change_price_providers(
        &self,
        price_providers: MultiValueEncoded<MultiValue2<TokenIdentifier, ManagedAddress>>,
    ) {
        self.price_providers().clear();

        for provider in price_providers.into_iter() {
            let tuple = provider.into_tuple();
            require!(tuple.0.is_valid_esdt_identifier(), INVALID_ESDT);
            require!(!tuple.1.is_zero(), ERROR_ZERO_VALUE);

            self.price_providers().insert(tuple.0, tuple.1);
        }
    }

    fn try_change_quorum(&self, new_value: BigUint) {
        require!(new_value != 0u64, ERROR_ZERO_VALUE);

        self.quorum().set(&new_value);
    }

    fn try_change_min_weight_for_proposal(&self, new_value: BigUint) {
        require!(new_value != 0u64, ERROR_ZERO_VALUE);

        self.min_weight_for_proposal().set(new_value);
    }

    fn try_change_voting_delay_in_blocks(&self, new_value: u64) {
        require!(new_value != 0, ERROR_ZERO_VALUE);

        self.voting_delay_in_blocks().set(new_value);
    }

    fn try_change_voting_period_in_blocks(&self, new_value: u64) {
        require!(new_value != 0, ERROR_ZERO_VALUE);

        self.voting_period_in_blocks().set(new_value);
    }

    #[view(getGovernanceTokenId)]
    #[storage_mapper("governanceTokenIds")]
    fn governance_token_ids(&self) -> SetMapper<TokenIdentifier>;

    #[view(getQuorum)]
    #[storage_mapper("quorum")]
    fn quorum(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinWeightForProposal)]
    #[storage_mapper("minWeightForProposal")]
    fn min_weight_for_proposal(&self) -> SingleValueMapper<BigUint>;

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

    #[storage_mapper("price_providers")]
    fn price_providers(&self) -> MapMapper<TokenIdentifier, ManagedAddress>;
}
