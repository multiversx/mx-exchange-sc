////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    farm::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    farm::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn acceptFee() {
    farm::endpoints::acceptFee(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn calculateRewardsForGivenPosition() {
    farm::endpoints::calculateRewardsForGivenPosition(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn claimRewards() {
    farm::endpoints::claimRewards(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn compoundRewards() {
    farm::endpoints::compoundRewards(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn end_produce_rewards() {
    farm::endpoints::end_produce_rewards(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn enterFarm() {
    farm::endpoints::enterFarm(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn enterFarmAndLockRewards() {
    farm::endpoints::enterFarmAndLockRewards(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn exitFarm() {
    farm::endpoints::exitFarm(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmount() {
    farm::endpoints::getBurnedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmountList() {
    farm::endpoints::getBurnedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getCurrentBlockFee() {
    farm::endpoints::getCurrentBlockFee(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getDivisionSafetyConstant() {
    farm::endpoints::getDivisionSafetyConstant(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFarmTokenId() {
    farm::endpoints::getFarmTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFarmTokenSupply() {
    farm::endpoints::getFarmTokenSupply(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFarmingTokenId() {
    farm::endpoints::getFarmingTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFarmingTokenReserve() {
    farm::endpoints::getFarmingTokenReserve(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmount() {
    farm::endpoints::getGeneratedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmountList() {
    farm::endpoints::getGeneratedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastErrorMessage() {
    farm::endpoints::getLastErrorMessage(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastRewardBlockNonce() {
    farm::endpoints::getLastRewardBlockNonce(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLockedAssetFactoryManagedAddress() {
    farm::endpoints::getLockedAssetFactoryManagedAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLockedRewardAprMuliplier() {
    farm::endpoints::getLockedRewardAprMuliplier(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getMinimumFarmingEpoch() {
    farm::endpoints::getMinimumFarmingEpoch(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getOwner() {
    farm::endpoints::getOwner(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPairContractManagedAddress() {
    farm::endpoints::getPairContractManagedAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPenaltyPercent() {
    farm::endpoints::getPenaltyPercent(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPerBlockRewardAmount() {
    farm::endpoints::getPerBlockRewardAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRewardPerShare() {
    farm::endpoints::getRewardPerShare(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRewardReserve() {
    farm::endpoints::getRewardReserve(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRewardTokenId() {
    farm::endpoints::getRewardTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRouterManagedAddress() {
    farm::endpoints::getRouterManagedAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getState() {
    farm::endpoints::getState(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTransferExecGasLimit() {
    farm::endpoints::getTransferExecGasLimit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getUndistributedFees() {
    farm::endpoints::getUndistributedFees(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn mergeFarmTokens() {
    farm::endpoints::mergeFarmTokens(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn pause() {
    farm::endpoints::pause(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn registerFarmToken() {
    farm::endpoints::registerFarmToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn resume() {
    farm::endpoints::resume(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLocalRolesFarmToken() {
    farm::endpoints::setLocalRolesFarmToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setPerBlockRewardAmount() {
    farm::endpoints::setPerBlockRewardAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_locked_rewards_apr_multiplier() {
    farm::endpoints::set_locked_rewards_apr_multiplier(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_minimum_farming_epochs() {
    farm::endpoints::set_minimum_farming_epochs(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_penalty_percent() {
    farm::endpoints::set_penalty_percent(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_transfer_exec_gas_limit() {
    farm::endpoints::set_transfer_exec_gas_limit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn start_produce_rewards() {
    farm::endpoints::start_produce_rewards(elrond_wasm_node::vm_api());
}
