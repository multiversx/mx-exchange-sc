/*
use elrond_wasm::types::Address;
use elrond_wasm_debug::{
    rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, StateChange},
    DebugApi,
};
use farm::*;
use pair::*;

const FARM_WASM_PATH: &'static str = "farm/ouput/farm.wasm";
const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";

const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
const LPTOK_TOKEN_ID: &[u8] = b"LPTOK-abcdef";
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;

struct FarmSetup<PairObjBuilder, FarmObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    pub wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
}

impl<PairObjBuilder, FarmObjBuilder> FarmSetup<PairObjBuilder, FarmObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    pub fn into_fields(
        self,
    ) -> (
        BlockchainStateWrapper,
        Address,
        ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
        ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    ) {
        (
            self.wrapper,
            self.owner_address,
            self.pair_wrapper,
            self.farm_wrapper,
        )
    }
}

// TODO: Also setup pair
fn setup_pair_and_farm<PairObjBuilder, FarmObjBuilder>(
    pair_builder: PairObjBuilder,
    farm_builder: FarmObjBuilder,
) -> FarmSetup<PairObjBuilder, FarmObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut wrapper = BlockchainStateWrapper::new();
    let owner_addr = wrapper.create_user_account(&rust_zero);
    let pair_wrapper =
        wrapper.create_sc_account(&rust_zero, Some(&owner_addr), pair_builder, PAIR_WASM_PATH);
    let farm_wrapper =
        wrapper.create_sc_account(&rust_zero, Some(&owner_addr), farm_builder, FARM_WASM_PATH);

    // init pair contract

    wrapper.execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
        // sc.init();

        StateChange::Commit
    });

    // init farm contract

    wrapper.execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
        // sc.init();

        StateChange::Commit
    });

    FarmSetup {
        wrapper,
        owner_address: owner_addr,
        pair_wrapper,
        farm_wrapper,
    }
}

#[test]
fn test_farm_pair_setup() {
    let _ = setup_pair_and_farm(pair::contract_obj, farm::contract_obj).into_fields();
}
*/
