////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    distribution::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    distribution::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn calculateLockedAssets() {
    distribution::endpoints::calculateLockedAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn claimLockedAssets() {
    distribution::endpoints::claimLockedAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn clearUnclaimableAssets() {
    distribution::endpoints::clearUnclaimableAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn deleteUserDistributedLockedAssets() {
    distribution::endpoints::deleteUserDistributedLockedAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn endGlobalOperation() {
    distribution::endpoints::endGlobalOperation(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAssetTokenId() {
    distribution::endpoints::getAssetTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getCommunityDistributionList() {
    distribution::endpoints::getCommunityDistributionList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastCommunityDistributionAmountAndEpoch() {
    distribution::endpoints::getLastCommunityDistributionAmountAndEpoch(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getUnlockPeriod() {
    distribution::endpoints::getUnlockPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getUsersDistributedLockedAssets() {
    distribution::endpoints::getUsersDistributedLockedAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getUsersDistributedLockedAssetsLength() {
    distribution::endpoints::getUsersDistributedLockedAssetsLength(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setCommunityDistribution() {
    distribution::endpoints::setCommunityDistribution(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setPerUserDistributedLockedAssets() {
    distribution::endpoints::setPerUserDistributedLockedAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setUnlockPeriod() {
    distribution::endpoints::setUnlockPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn startGlobalOperation() {
    distribution::endpoints::startGlobalOperation(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn undoLastCommunityDistribution() {
    distribution::endpoints::undoLastCommunityDistribution(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn undoUserDistributedAssetsBetweenEpochs() {
    distribution::endpoints::undoUserDistributedAssetsBetweenEpochs(elrond_wasm_node::vm_api());
}
