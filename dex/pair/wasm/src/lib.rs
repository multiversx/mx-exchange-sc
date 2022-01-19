////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    pair
    (
        init
        addInitialLiquidity
        addLiquidity
        addTrustedSwapPair
        getAmountIn
        getAmountOut
        getEquivalent
        getExternSwapGasLimit
        getFeeDestinations
        getFeeState
        getFirstTokenId
        getInitialLiquidtyAdder
        getLpTokenIdentifier
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
        setFeeOn
        setFeePercents
        setLpTokenIdentifier
        setStateActiveNoSwaps
        set_extern_swap_gas_limit
        set_transfer_exec_gas_limit
        swapNoFeeAndForward
        swapTokensFixedInput
        swapTokensFixedOutput
        whitelist
    )
}

elrond_wasm_node::wasm_empty_callback! {}
