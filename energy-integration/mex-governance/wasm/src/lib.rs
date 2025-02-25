// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Upgrade:                              1
// Endpoints:                           14
// Async Callback (empty):               1
// Total number of exported functions:  17

#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    mex_governance
    (
        init => init
        upgrade => upgrade
        whitelistFarms => whitelist_farms
        removeWhitelistFarm => remove_whitelist_farm
        blacklistFarm => blacklist_farm
        setReferenceEmissionRate => set_reference_emission_rate
        setIncentiveToken => set_incentive_token
        setFarmEmissions => set_farm_emissions
        incentivizeFarm => incentivize_farm
        claimIncentive => claim_incentive
        vote => vote
        setEnergyFactoryAddress => set_energy_factory_address
        getEnergyFactoryAddress => energy_factory_address
        getCurrentWeek => get_current_week
        getFirstWeekStartEpoch => first_week_start_epoch
        getAllWeekEmissions => get_all_week_emissions
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
