////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]

elrond_wasm_node::wasm_endpoints! {
    farm
    (
        init
        callBack
        calculateRewardsForGivenPosition
        claimRewards
        compoundRewards
        end_produce_rewards
        enterFarm
        exitFarm
        getBurnGasLimit
        getDivisionSafetyConstant
        getFarmTokenId
        getFarmTokenSupply
        getFarmingTokenId
        getLastErrorMessage
        getLastRewardBlockNonce
        getMinimumFarmingEpoch
        getOwner
        getPairContractManagedAddress
        getPenaltyPercent
        getPerBlockRewardAmount
        getRewardPerShare
        getRewardReserve
        getRewardTokenId
        getState
        getTransferExecGasLimit
        mergeFarmTokens
        pause
        registerFarmToken
        resume
        setLocalRolesFarmToken
        setPerBlockRewardAmount
        set_burn_gas_limit
        set_minimum_farming_epochs
        set_penalty_percent
        set_transfer_exec_gas_limit
        startProduceRewards
    )
}
