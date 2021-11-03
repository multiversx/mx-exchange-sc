////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    factory::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    factory::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn createAndForward() {
    factory::endpoints::createAndForward(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn createAndForwardCustomPeriod() {
    factory::endpoints::createAndForwardCustomPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAssetTokenId() {
    factory::endpoints::getAssetTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmount() {
    factory::endpoints::getBurnedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmountList() {
    factory::endpoints::getBurnedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getCacheSize() {
    factory::endpoints::getCacheSize(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getDefaultUnlockPeriod() {
    factory::endpoints::getDefaultUnlockPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmount() {
    factory::endpoints::getGeneratedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmountList() {
    factory::endpoints::getGeneratedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getInitEpoch() {
    factory::endpoints::getInitEpoch(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastErrorMessage() {
    factory::endpoints::getLastErrorMessage(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLockedAssetTokenId() {
    factory::endpoints::getLockedAssetTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTransferExecGasLimit() {
    factory::endpoints::getTransferExecGasLimit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getUnlockScheduleForSFTNonce() {
    factory::endpoints::getUnlockScheduleForSFTNonce(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getWhitelistedContracts() {
    factory::endpoints::getWhitelistedContracts(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn mergeLockedAssetTokens() {
    factory::endpoints::mergeLockedAssetTokens(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn registerLockedAssetToken() {
    factory::endpoints::registerLockedAssetToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeWhitelist() {
    factory::endpoints::removeWhitelist(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLocalRolesLockedAssetToken() {
    factory::endpoints::setLocalRolesLockedAssetToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setUnlockPeriod() {
    factory::endpoints::setUnlockPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_transfer_exec_gas_limit() {
    factory::endpoints::set_transfer_exec_gas_limit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn unlockAssets() {
    factory::endpoints::unlockAssets(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn whitelist() {
    factory::endpoints::whitelist(elrond_wasm_node::vm_api());
}
