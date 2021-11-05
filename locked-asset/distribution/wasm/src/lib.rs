////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
   distribution
   (
        init
        callBack
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
