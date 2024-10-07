// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Upgrade:                              1
// Endpoints:                           10
// Async Callback (empty):               1
// Total number of exported functions:  13

#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    metabonding_staking_legacy
    (
        init => init
        upgrade => upgrade
        stakeLockedAsset => stake_locked_asset
        unstake => unstake
        unbond => unbond
        getStakedAmountForUser => get_staked_amount_for_user
        getUserEntry => get_user_entry
        getSnapshot => get_snapshot
        getLockedAssetTokenId => locked_asset_token_id
        getLockedAssetFactoryAddress => locked_asset_factory_address
        getTotalLockedAssetSupply => total_locked_asset_supply
        getUserList => user_list
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
