// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Upgrade:                              1
// Endpoints:                           33
// Async Callback:                       1
// Total number of exported functions:  36

#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    proxy_dex
    (
        init => init
        upgrade => upgrade
        registerProxyPair => register_proxy_pair
        setTransferRoleWrappedLpToken => set_transfer_role_wrapped_lp_token
        registerProxyFarm => register_proxy_farm
        setTransferRoleWrappedFarmToken => set_transfer_role_wrapped_farm_token
        getAssetTokenId => get_asset_token_id_view
        getLockedTokenIds => get_locked_token_ids_view
        getOldLockedTokenId => old_locked_token_id
        getOldFactoryAddress => old_factory_address
        getWrappedLpTokenId => wrapped_lp_token
        getWrappedFarmTokenId => wrapped_farm_token
        addPairToIntermediate => add_pair_to_intermediate
        removeIntermediatedPair => remove_intermediated_pair
        addFarmToIntermediate => add_farm_to_intermediate
        removeIntermediatedFarm => remove_intermediated_farm
        getIntermediatedPairs => intermediated_pairs
        getIntermediatedFarms => intermediated_farms
        addLiquidityProxy => add_liquidity_proxy
        removeLiquidityProxy => remove_liquidity_proxy
        increaseProxyPairTokenEnergy => increase_proxy_pair_token_energy_endpoint
        enterFarmProxy => enter_farm_proxy_endpoint
        exitFarmProxy => exit_farm_proxy
        claimRewardsProxy => claim_rewards_proxy
        increaseProxyFarmTokenEnergy => increase_proxy_farm_token_energy_endpoint
        mergeWrappedFarmTokens => merge_wrapped_farm_tokens_endpoint
        mergeWrappedLpTokens => merge_wrapped_lp_tokens_endpoint
        setEnergyFactoryAddress => set_energy_factory_address
        getEnergyFactoryAddress => energy_factory_address
        addSCAddressToWhitelist => add_sc_address_to_whitelist
        removeSCAddressFromWhitelist => remove_sc_address_from_whitelist
        isSCAddressWhitelisted => is_sc_address_whitelisted
        enableAddLiq => enable_add_liq
        disableAddLiq => disable_add_liq
        isAddLiqDisabled => add_liq_disabled
    )
}

multiversx_sc_wasm_adapter::async_callback! { proxy_dex }
