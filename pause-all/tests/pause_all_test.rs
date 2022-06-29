use elrond_wasm::types::Address;
use elrond_wasm_debug::{rust_biguint, testing_framework::*, DebugApi};
use pause_all::*;

const WASM_PATH: &'static str = "output/pause-all.wasm";

#[test]
fn pause_all_test() {
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_address = b_mock.create_user_account(&rust_zero);
    let pause_sc = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        pause_all::contract_obj,
        WASM_PATH,
    );

    b_mock
        .execute_tx(&owner_address, &pause_sc, &rust_zero, |sc| {
            sc.init();
        })
        .assert_ok();

    // simulate deploy
    b_mock
        .execute_tx(&owner_address, &pause_sc, &rust_biguint!(0u64), |sc| {
            sc.init();
        })
        .assert_ok();
}
