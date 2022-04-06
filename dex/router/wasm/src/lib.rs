////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    router
    (
        callBack
        clearPairTemporaryOwnerStorage
        createPair
        getAllPairContractMetadata
        getAllPairTokens
        getAllPairsManagedAddresses
        getLastErrorMessage
        getOwner
        getPair
        getPairCreationEnabled
        getPairTemplateAddress
        getState
        getTemporaryOwnerPeriod
        getTransferExecGasLimit
        issueLpToken
        multiPairSwap
        pairSetLockingDeadlineEpoch
        pairSetLockingScAddress
        pairSetUnlockEpoch
        pause
        removePair
        resume
        setFeeOff
        setFeeOn
        setLocalRoles
        setLocalRolesOwner
        setPairCreationEnabled
        setPairTemplateAddress
        setTemporaryOwnerPeriod
        upgradePair
    )
}
