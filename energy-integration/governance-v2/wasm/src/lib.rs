////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    governance_v2
    (
        cancel
        changeLockTimeAfterVotingEndsInBlocks
        changeMinEnergyForProposal
        changeMinFeeForProposal
        changeQuorum
        changeVotingDelayInBlocks
        changeVotingPeriodInBlocks
        claimDepositedTokens
        depositTokensForProposal
        execute
        getEnergyFactoryAddress
        getFeeTokenId
        getLockTimeAfterVotingEndsInBlocks
        getMinEnergyForPropose
        getMinFeeForPropose
        getProposalActions
        getProposalDescription
        getProposalStatus
        getProposalVotes
        getProposer
        getQuorum
        getVotingDelayInBlocks
        getVotingPeriodInBlocks
        propose
        queue
        setEnergyFactoryAddress
        vote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
