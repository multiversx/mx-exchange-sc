////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    governance
    (
        changeMinTokenBalanceForProposing
        changeQuorum
        changeVotingDelayInBlocks
        changeVotingPeriodInBlocks
        downvote
        execute
        getGovernanceTokenId
        getMexTokenId
        getMinWeightForProposal
        getProposal
        getProposalIdCounter
        getProposalStatus
        getQuorum
        getVoteNFTId
        getVotingDelayInBlocks
        getVotingPeriodInBlocks
        propose
        reclaimTokens
        upvote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
