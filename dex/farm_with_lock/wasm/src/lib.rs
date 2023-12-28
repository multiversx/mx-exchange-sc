////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    farm_with_lock
    (
        callBack
        calculateRewardsForGivenPosition
        end_produce_rewards
        exitFarm
        getBurnGasLimit
        getDivisionSafetyConstant
        getFarmMigrationConfiguration
        getFarmTokenId
        getFarmTokenSupply
        getFarmingTokenId
        getLastErrorMessage
        getLastRewardBlockNonce
        getLockedAssetFactoryManagedAddress
        getMinimumFarmingEpoch
        getPairContractManagedAddress
        getPenaltyPercent
        getPerBlockRewardAmount
        getRewardPerShare
        getRewardReserve
        getRewardTokenId
        getState
        getTransferExecGasLimit
        migrateFromV1_2Farm
        pause
        registerFarmToken
        resume
        setFarmMigrationConfig
        setFarmTokenSupply
        setLocalRolesFarmToken
        setRpsAndStartRewards
        setTransferRoleFarmToken
        set_burn_gas_limit
        set_minimum_farming_epochs
        set_penalty_percent
        set_transfer_exec_gas_limit
        startProduceRewards
    )
}
