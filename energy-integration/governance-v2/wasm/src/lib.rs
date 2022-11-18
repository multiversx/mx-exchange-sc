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
        changeQuorum
        changeVotingDelayInBlocks
        changeVotingPeriodInBlocks
        depositTokensForProposal
        execute
        getEnergyFactoryAddress
        getLockTimeAfterVotingEndsInBlocks
        getMinEnergyForPropose
        getProposalActions
        getProposalDescription
        getProposalRootHash
        getProposalStatus
        getProposalVotes
        getProposer
        getQuorum
        getVoteStatus
        getVotingDelayInBlocks
        getVotingPeriodInBlocks
        propose
        queue
        setEnergyFactoryAddress
        vote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
