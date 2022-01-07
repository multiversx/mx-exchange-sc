use elrond_wasm::types::{Address, EsdtLocalRole, ManagedAddress, SCResult, TokenIdentifier};
use elrond_wasm_debug::{
    assert_sc_error, managed_address, managed_token_id, rust_biguint, DebugApi,
};
use elrond_wasm_debug::{managed_biguint, testing_framework::*};
use num_traits::ToPrimitive;
use price_discovery::common_storage::*;
use price_discovery::create_pool::*;
use price_discovery::redeem_token::*;
use price_discovery::*;

mod tests_common;
use tests_common::*;

#[test]
fn test_init() {
    let _ = init(price_discovery::contract_obj, pair_mock::contract_obj);
}

#[test]
fn test_deposit_initial_tokens_too_late() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    pd_setup.blockchain_wrapper.set_block_epoch(5);

    let sc_result = call_deposit_initial_tokens(
        &mut pd_setup,
        &rust_biguint!(5_000_000_000),
        StateChange::Revert,
    );
    assert_sc_error!(sc_result, b"May only deposit before start epoch");
}

#[test]
fn test_deposit_initial_tokens_ok() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);

    let sc_result = call_deposit_initial_tokens(
        &mut pd_setup,
        &rust_biguint!(5_000_000_000),
        StateChange::Revert,
    );
    assert_eq!(sc_result, SCResult::Ok(()));
}
