////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    energy_factory
    (
        callBack
        addLockOptions
        addSCAddressToWhitelist
        getBaseAssetTokenId
        getEnergyAmountForUser
        getEnergyEntryForUser
        getFeesBurnPercentage
        getFeesCollectorAddress
        getFeesFromPenaltyUnlocking
        getLastEpochFeeSentToCollector
        getLegacyLockedTokenId
        getLockOptions
        getLockedTokenId
        getPenaltyAmount
        isPaused
        isSCAddressWhitelisted
        issueLockedToken
        lockTokens
        lockVirtual
        mergeTokens
        pause
        reduceLockPeriod
        removeLockOptions
        removeSCAddressFromWhitelist
        setFeesBurnPercentage
        setFeesCollectorAddress
        setTransferRoleLockedToken
        unlockEarly
        unlockTokens
        unpause
        updateEnergyAfterOldTokenUnlock
        updateEnergyForOldTokens
    )
}
