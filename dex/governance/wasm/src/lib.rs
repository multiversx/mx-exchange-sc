////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    governance
    (
        changeMaxActionsPerProposal
        changeMinTokenBalanceForProposing
        changeQuorum
        changeVotingDelayInBlocks
        changeVotingPeriodInBlocks
        downvote
        execute
        getGovernanceTokenId
        getMaxActionsPerProposal
        getMexTokenId
        getMinTokenBalanceForProposing
        getProposal
        getProposalIdCounter
        getProposalStatus
        getQuorum
        getVoteNFTId
        getVotingDelayInBlocks
        getVotingPeriodInBlocks
        propose
        vote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
