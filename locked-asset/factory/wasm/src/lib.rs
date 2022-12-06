////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    factory
    (
        createAndForward
        createAndForwardCustomPeriod
        getAssetTokenId
        getCacheSize
        getDefaultUnlockPeriod
        getExtendedAttributesActivationNonce
        getInitEpoch
        getLastErrorMessage
        getLockedAssetTokenId
        getUnlockScheduleForSFTNonce
        getWhitelistedContracts
        isPaused
        pause
        removeWhitelist
        setBurnRoleForAddress
        setNewFactoryAddress
        setTransferRoleForAddress
        unlockAssets
        unpause
        unsetTransferRoleForAddress
        whitelist
    )
}

elrond_wasm_node::wasm_empty_callback! {}
