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
        getAllUsersDistributedLockedAssets
        getAssetTokenId
        getCommunityDistributionList
        getLastCommunityDistributionAmountAndEpoch
        getUnlockPeriod
        setCommunityDistribution
        setPerUserDistributedLockedAssets
        setUnlockPeriod
        startGlobalOperation
        undoLastCommunityDistribution
        undoUserDistributedAssetsBetweenEpochs
    )
}

elrond_wasm_node::wasm_empty_callback! {}
