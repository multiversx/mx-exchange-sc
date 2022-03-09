////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    pair
    (
        addInitialLiquidity
        addLiquidity
        addTrustedSwapPair
        getAmountIn
        getAmountOut
        getBPAddConfig
        getBPRemoveConfig
        getBPSwapConfig
        getEquivalent
        getExternSwapGasLimit
        getFeeDestinations
        getFeeState
        getFirstTokenId
        getInitialLiquidtyAdder
        getLpTokenIdentifier
        getNumAddsByAddress
        getNumRemovesByAddress
        getNumSwapsByAddress
        getReserve
        getReservesAndTotalSupply
        getRouterManagedAddress
        getRouterOwnerManagedAddress
        getSecondTokenId
        getSpecialFee
        getState
        getTokensForGivenPosition
        getTotalFeePercent
        getTotalSupply
        getTransferExecGasLimit
        getTrustedSwapPairs
        getWhitelistedManagedAddresses
        pause
        removeLiquidity
        removeLiquidityAndBuyBackAndBurnToken
        removeTrustedSwapPair
        removeWhitelist
        resume
        setBPAddConfig
        setBPRemoveConfig
        setBPSwapConfig
        setFeeOn
        setFeePercents
        setLpTokenIdentifier
        setMaxObservationsPerRecord
        setStateActiveNoSwaps
        set_extern_swap_gas_limit
        set_transfer_exec_gas_limit
        swapNoFeeAndForward
        swapTokensFixedInput
        swapTokensFixedOutput
        updateAndGetSafePrice
        updateAndGetTokensForGivenPositionWithSafePrice
        whitelist
    )
}

elrond_wasm_node::wasm_empty_callback! {}
