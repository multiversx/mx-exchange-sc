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
        getAllKnownContracts
        getAllTokens
        getCurrentClaimProgress
        getCurrentWeek
        getEnergyFactoryAddress
        getFirstWeekStartEpoch
        getLastActiveWeekForUser
        getTotalEnergyForWeek
        getTotalLockedTokensForWeek
        getTotalRewardsForWeek
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
