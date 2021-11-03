////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    proxy_dex::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    proxy_dex::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn acceptPay() {
    proxy_dex::endpoints::acceptPay(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn addFarmToIntermediate() {
    proxy_dex::endpoints::addFarmToIntermediate(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn addLiquidityProxy() {
    proxy_dex::endpoints::addLiquidityProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn addPairToIntermediate() {
    proxy_dex::endpoints::addPairToIntermediate(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn claimRewardsProxy() {
    proxy_dex::endpoints::claimRewardsProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn compoundRewardsProxy() {
    proxy_dex::endpoints::compoundRewardsProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn enterFarmAndLockRewardsProxy() {
    proxy_dex::endpoints::enterFarmAndLockRewardsProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn enterFarmProxy() {
    proxy_dex::endpoints::enterFarmProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn exitFarmProxy() {
    proxy_dex::endpoints::exitFarmProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAssetTokenId() {
    proxy_dex::endpoints::getAssetTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmount() {
    proxy_dex::endpoints::getBurnedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmountList() {
    proxy_dex::endpoints::getBurnedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmount() {
    proxy_dex::endpoints::getGeneratedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmountList() {
    proxy_dex::endpoints::getGeneratedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getIntermediatedFarms() {
    proxy_dex::endpoints::getIntermediatedFarms(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getIntermediatedPairs() {
    proxy_dex::endpoints::getIntermediatedPairs(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastErrorMessage() {
    proxy_dex::endpoints::getLastErrorMessage(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLockedAssetTokenId() {
    proxy_dex::endpoints::getLockedAssetTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTransferExecGasLimit() {
    proxy_dex::endpoints::getTransferExecGasLimit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getWrappedFarmTokenId() {
    proxy_dex::endpoints::getWrappedFarmTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getWrappedLpTokenId() {
    proxy_dex::endpoints::getWrappedLpTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn mergeWrappedFarmTokens() {
    proxy_dex::endpoints::mergeWrappedFarmTokens(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn mergeWrappedLpTokens() {
    proxy_dex::endpoints::mergeWrappedLpTokens(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn registerProxyFarm() {
    proxy_dex::endpoints::registerProxyFarm(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn registerProxyPair() {
    proxy_dex::endpoints::registerProxyPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeIntermediatedFarm() {
    proxy_dex::endpoints::removeIntermediatedFarm(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeIntermediatedPair() {
    proxy_dex::endpoints::removeIntermediatedPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeLiquidityProxy() {
    proxy_dex::endpoints::removeLiquidityProxy(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLocalRoles() {
    proxy_dex::endpoints::setLocalRoles(elrond_wasm_node::vm_api());
}
