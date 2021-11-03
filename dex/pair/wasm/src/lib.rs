////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

#![no_std]
#![allow(non_snake_case)]

pub use elrond_wasm_output;

#[no_mangle]
pub fn init() {
    pair::endpoints::init(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn callBack() {
    pair::endpoints::callBack(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn addLiquidity() {
    pair::endpoints::addLiquidity(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn addTrustedSwapPair() {
    pair::endpoints::addTrustedSwapPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAmountIn() {
    pair::endpoints::getAmountIn(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getAmountOut() {
    pair::endpoints::getAmountOut(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmount() {
    pair::endpoints::getBurnedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getBurnedTokenAmountList() {
    pair::endpoints::getBurnedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getEquivalent() {
    pair::endpoints::getEquivalent(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getExternSwapGasLimit() {
    pair::endpoints::getExternSwapGasLimit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFeeDestinations() {
    pair::endpoints::getFeeDestinations(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFeeState() {
    pair::endpoints::getFeeState(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getFirstTokenId() {
    pair::endpoints::getFirstTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmount() {
    pair::endpoints::getGeneratedTokenAmount(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getGeneratedTokenAmountList() {
    pair::endpoints::getGeneratedTokenAmountList(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getLpTokenIdentifier() {
    pair::endpoints::getLpTokenIdentifier(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getReserve() {
    pair::endpoints::getReserve(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getReservesAndTotalSupply() {
    pair::endpoints::getReservesAndTotalSupply(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRouterManagedAddress() {
    pair::endpoints::getRouterManagedAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getRouterOwnerManagedAddress() {
    pair::endpoints::getRouterOwnerManagedAddress(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getSecondTokenId() {
    pair::endpoints::getSecondTokenId(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getSpecialFee() {
    pair::endpoints::getSpecialFee(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getState() {
    pair::endpoints::getState(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTokensForGivenPosition() {
    pair::endpoints::getTokensForGivenPosition(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTotalFeePercent() {
    pair::endpoints::getTotalFeePercent(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTotalSupply() {
    pair::endpoints::getTotalSupply(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTransferExecGasLimit() {
    pair::endpoints::getTransferExecGasLimit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getTrustedSwapPairs() {
    pair::endpoints::getTrustedSwapPairs(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn getWhitelistedManagedAddresses() {
    pair::endpoints::getWhitelistedManagedAddresses(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn pause() {
    pair::endpoints::pause(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeLiquidity() {
    pair::endpoints::removeLiquidity(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeLiquidityAndBuyBackAndBurnToken() {
    pair::endpoints::removeLiquidityAndBuyBackAndBurnToken(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeTrustedSwapPair() {
    pair::endpoints::removeTrustedSwapPair(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn removeWhitelist() {
    pair::endpoints::removeWhitelist(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn resume() {
    pair::endpoints::resume(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setFeeOn() {
    pair::endpoints::setFeeOn(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setFeePercents() {
    pair::endpoints::setFeePercents(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setLpTokenIdentifier() {
    pair::endpoints::setLpTokenIdentifier(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn setStateActiveNoSwaps() {
    pair::endpoints::setStateActiveNoSwaps(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_extern_swap_gas_limit() {
    pair::endpoints::set_extern_swap_gas_limit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn set_transfer_exec_gas_limit() {
    pair::endpoints::set_transfer_exec_gas_limit(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn swapNoFeeAndForward() {
    pair::endpoints::swapNoFeeAndForward(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn swapTokensFixedInput() {
    pair::endpoints::swapTokensFixedInput(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn swapTokensFixedOutput() {
    pair::endpoints::swapTokensFixedOutput(elrond_wasm_node::vm_api());
}

#[no_mangle]
pub fn whitelist() {
    pair::endpoints::whitelist(elrond_wasm_node::vm_api());
}
