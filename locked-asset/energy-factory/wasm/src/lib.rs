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
        addToTokenTransferWhitelist
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
        removeFromTokenTransferWhitelist
        removeSCAddressFromWhitelist
        revertUnstake
        setBurnRoleLockedToken
        setEnergyForOldTokens
        setTokenUnstakeAddress
        setTransferRoleLockedToken
        setUserEnergyAfterLockedTokenTransfer
        unlockEarly
        unlockTokens
        unpause
        updateEnergyAfterOldTokenUnlock
    )
}
