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
        getAccumulatedLockedFees
        getAllKnownContracts
        getAllTokens
        getCurrentClaimProgress
        getCurrentWeek
        getEnergyFactoryAddress
        getFirstWeekStartEpoch
        getLastActiveWeekForUser
        getLastGlobalUpdateWeek
        getTotalEnergyForWeek
        getTotalLockedTokensForWeek
        getTotalRewardsForWeek
        getUserEnergyForWeek
        isPaused
        pause
        removeKnownContracts
        removeKnownTokens
        setEnergyFactoryAddress
        unpause
        updateEnergyForUser
    )
}

elrond_wasm_node::wasm_empty_callback! {}
