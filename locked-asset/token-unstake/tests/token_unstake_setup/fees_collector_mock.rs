use elrond_wasm::contract_base::{CallableContract, ContractBase};
use elrond_wasm_debug::DebugApi;

static DEPOSIT_FN_NAME: &[u8] = b"depositSwapFees";

#[derive(Clone)]
pub struct FeesCollectorMock {}

impl ContractBase for FeesCollectorMock {
    type Api = DebugApi;
}

impl CallableContract for FeesCollectorMock {
    fn call(&self, fn_name: &[u8]) -> bool {
        fn_name == DEPOSIT_FN_NAME
    }
}

impl FeesCollectorMock {
    pub fn new() -> Self {
        FeesCollectorMock {}
    }
}
