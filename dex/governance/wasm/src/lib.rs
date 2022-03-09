////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    governance
    (
        changeGovernanceTokenIds
        changeMinTokenBalanceForProposing
        changePriceProviders
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
        redeem
        upvote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
