// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Upgrade:                              1
// Endpoints:                           11
// Async Callback (empty):               1
// Total number of exported functions:  14

#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    token_unstake
    (
        init => init
        upgrade => upgrade
        getUnbondEpochs => unbond_epochs
        getUnlockedTokensForUser => unlocked_tokens_for_user
        claimUnlockedTokens => claim_unlocked_tokens
        cancelUnbond => cancel_unbond
        depositUserTokens => deposit_user_tokens
        depositFees => deposit_fees
        setFeesBurnPercentage => set_fees_burn_percentage
        getFeesBurnPercentage => fees_burn_percentage
        getFeesCollectorAddress => fees_collector_address
        setEnergyFactoryAddress => set_energy_factory_address
        getEnergyFactoryAddress => energy_factory_address
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
