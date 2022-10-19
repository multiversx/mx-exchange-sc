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
        getCurrentGlobalActiveWeek
        getCurrentWeek
        getEnergyFactoryAddress
        getFirstWeekStartEpoch
        getLastActiveWeekForUser
        getLastGlobalActiveWeek
        getLastGlobalUpdateWeek
        getTotalEnergyForWeek
        getTotalLockedTokensForWeek
        getUserEnergyForWeek
        isPaused
        pause
        recomputeEnergy
        removeKnownContracts
        removeKnownTokens
        setEnergyFactoryAddress
        unpause
    )
}

elrond_wasm_node::wasm_empty_callback! {}
