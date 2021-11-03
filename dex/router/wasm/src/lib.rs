////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    router::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    router::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn clearPairTemporaryOwnerStorage() {
    router::endpoints::clearPairTemporaryOwnerStorage(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn createPair() {
    router::endpoints::createPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAllPairContractMetadata() {
    router::endpoints::getAllPairContractMetadata(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAllPairTokens() {
    router::endpoints::getAllPairTokens(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAllPairsManagedAddresses() {
    router::endpoints::getAllPairsManagedAddresses(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLastErrorMessage() {
    router::endpoints::getLastErrorMessage(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getOwner() {
    router::endpoints::getOwner(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPair() {
    router::endpoints::getPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPairCreationEnabled() {
    router::endpoints::getPairCreationEnabled(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getPairTemplateAddress() {
    router::endpoints::getPairTemplateAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getState() {
    router::endpoints::getState(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTemporaryOwnerPeriod() {
    router::endpoints::getTemporaryOwnerPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn issueLpToken() {
    router::endpoints::issueLpToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn pause() {
    router::endpoints::pause(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn resume() {
    router::endpoints::resume(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setFeeOff() {
    router::endpoints::setFeeOff(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setFeeOn() {
    router::endpoints::setFeeOn(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLocalRoles() {
    router::endpoints::setLocalRoles(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLocalRolesOwner() {
    router::endpoints::setLocalRolesOwner(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setPairCreationEnabled() {
    router::endpoints::setPairCreationEnabled(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setPairTemplateAddress() {
    router::endpoints::setPairTemplateAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setTemporaryOwnerPeriod() {
    router::endpoints::setTemporaryOwnerPeriod(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn upgradePair() {
    router::endpoints::upgradePair(elrond_wasm_node::vm_api());
}
