////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    proxy_dex
    (
        addFarmToIntermediate
        addPairToIntermediate
        exitFarmProxy
        getAssetTokenId
        getIntermediatedFarms
        getIntermediatedPairs
        getLockedAssetTokenId
        getWrappedFarmTokenId
        getWrappedLpTokenId
        migrateV1_2Position
        removeIntermediatedFarm
        removeIntermediatedPair
        removeLiquidityProxy
        setTransferRoleLockedFarmToken
        setTransferRoleLockedLpToken
        unsetTransferRoleLockedFarmToken
        unsetTransferRoleLockedLpToken
    )
}

elrond_wasm_node::wasm_empty_callback! {}
