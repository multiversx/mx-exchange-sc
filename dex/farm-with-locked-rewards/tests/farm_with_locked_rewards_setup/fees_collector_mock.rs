use multiversx_sc::contract_base::{CallableContract, ContractBase};
use multiversx_sc_scenario::DebugApi;

static DEPOSIT_FN_NAME: &str = "depositSwapFees";

#[derive(Clone)]
pub struct FeesCollectorMock {}

impl ContractBase for FeesCollectorMock {
    type Api = DebugApi;
}

impl CallableContract for FeesCollectorMock {
    fn call(&self, fn_name: &str) -> bool {
        fn_name == DEPOSIT_FN_NAME
    }
}

impl FeesCollectorMock {
    pub fn new() -> Self {
        FeesCollectorMock {}
    }
}
