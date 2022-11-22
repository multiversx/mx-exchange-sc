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
        createMergedLockedTokenForFees
        getBaseAssetTokenId
        getEnergyAmountForUser
        getEnergyEntryForUser
        getLegacyLockedTokenId
        getLockOptions
        getLockedTokenId
        getPenaltyAmount
        getTokenUnstakeScAddress
        isPaused
        isSCAddressWhitelisted
        issueLockedToken
        lockTokens
        lockVirtual
        mergeTokens
        migrateOldTokens
        pause
        reduceLockPeriod
        removeSCAddressFromWhitelist
        revertUnstake
        setBurnRoleLockedToken
        setLockedTokenTransferScAddress
        setTokenUnstakeAddress
        setTransferRoleLockedToken
        setUserEnergyAfterLockedTokenTransfer
        unlockEarly
        unlockTokens
        unpause
        updateEnergyAfterOldTokenUnlock
        updateEnergyForOldTokens
    )
}
