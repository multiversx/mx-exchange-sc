////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    fees_collector
    (
        addKnownContracts
        addKnownTokens
        claimRewards
        depositSwapFees
        getAccumulatedFees
        getAllKnownContracts
        getAllTokens
        getCurrentClaimProgress
        getCurrentWeek
        getEnergyFactoryAddress
        getFirstWeekStartEpoch
        getLastActiveWeekForUser
        getLastGlobalUpdateWeek
        getLockEpochs
        getLockedTokenId
        getLockingScAddress
        getTotalEnergyForWeek
        getTotalLockedTokensForWeek
        getTotalRewardsForWeek
        getUserEnergyForWeek
        isPaused
        pause
        removeKnownContracts
        removeKnownTokens
        setEnergyFactoryAddress
        setLockEpochs
        setLockingScAddress
        unpause
        updateEnergyForUser
    )
}

elrond_wasm_node::wasm_empty_callback! {}
