////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    distribution
    (
        init
        calculateLockedAssets
        claimLockedAssets
        clearUnclaimableAssets
        deleteUserDistributedLockedAssets
        endGlobalOperation
        getAssetTokenId
        getCommunityDistributionList
        getLastCommunityDistributionAmountAndEpoch
        getUnlockPeriod
        getUsersDistributedLockedAssets
        getUsersDistributedLockedAssetsLength
        setCommunityDistribution
        setPerUserDistributedLockedAssets
        setUnlockPeriod
        startGlobalOperation
        undoLastCommunityDistribution
        undoUserDistributedAssetsBetweenEpochs
    )
}

elrond_wasm_node::wasm_empty_callback! {}
