////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

multiversx_sc_wasm_adapter::wasm_endpoints! {
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

multiversx_sc_wasm_adapter::wasm_empty_callback! {}
