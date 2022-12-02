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
        getEnergyFactoryAddress
        getExtendedAttributesActivationNonce
        getIntermediatedFarms
        getIntermediatedPairs
        getLockedAssetTokenId
        getWrappedFarmTokenId
        getWrappedLpTokenId
        migrateV1_2Position
        removeIntermediatedFarm
        removeIntermediatedPair
        removeLiquidityProxy
        setEnergyFactoryAddress
        setTransferRoleLockedFarmToken
        setTransferRoleLockedLpToken
        unsetTransferRoleLockedFarmToken
        unsetTransferRoleLockedLpToken
    )
}

elrond_wasm_node::wasm_empty_callback! {}
