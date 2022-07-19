////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    governance_v2
    (
        cancel
        changeLockTimeAfterVotingEndsInBlocks
        changeQuorum
        changeVotingDelayInBlocks
        changeVotingPeriodInBlocks
        depositTokensForAction
        downvote
        execute
        getEnergyFactoryAddress
        getLockTimeAfterVotingEndsInBlocks
        getProposalActions
        getProposalDescription
        getProposalStatus
        getProposer
        getQuorum
        getTotalDownvotes
        getTotalVotes
        getVotingDelayInBlocks
        getVotingPeriodInBlocks
        propose
        queue
        setEnergyFactoryAddress
        vote
    )
}

elrond_wasm_node::wasm_empty_callback! {}
