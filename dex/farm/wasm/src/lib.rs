////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    farm
    (
        init
        acceptFee
        calculateRewardsForGivenPosition
        end_produce_rewards_as_owner
        exitFarm
        getBurnedTokenAmount
        getCurrentBlockFee
        getDivisionSafetyConstant
        getFarmTokenId
        getFarmTokenSupply
        getFarmingTokenId
        getFarmingTokenReserve
        getLastErrorMessage
        getLastRewardBlockNonce
        getLockedAssetFactoryManagedAddress
        getLockedRewardAprMuliplier
        getMinimumFarmingEpoch
        getOwner
        getPairContractManagedAddress
        getPenaltyPercent
        getPerBlockRewardAmount
        getRewardPerShare
        getRewardReserve
        getRewardTokenId
        getRouterManagedAddress
        getState
        getTransferExecGasLimit
        getUndistributedFees
        pause
        resume
        setPerBlockRewardAmount
        setTransferRoleFarmToken
        set_locked_rewards_apr_multiplier
        set_minimum_farming_epochs
        set_penalty_percent
        set_transfer_exec_gas_limit
        start_produce_rewards
    )
}

elrond_wasm_node::wasm_empty_callback! {}
