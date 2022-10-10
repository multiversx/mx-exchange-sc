use elrond_wasm::types::{EsdtLocalRole, ManagedAddress};
use elrond_wasm_debug::testing_framework::*;
use elrond_wasm_debug::{managed_token_id, rust_biguint};

use lkmex_transfer::LkmexTransfer;

const LOCKED_TOKEN_ID: &[u8] = b"NOOO0-123456";

#[test]
fn lock_unlock_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let claimer_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        lkmex_transfer::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.init(managed_token_id!(LOCKED_TOKEN_ID), 4, 6);
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[EsdtLocalRole::Transfer],
    );

    let balance_amount = rust_biguint!(2_000);
    let lock_amount = rust_biguint!(1_000);
    b_mock.set_esdt_balance(&user_addr, LOCKED_TOKEN_ID, &balance_amount);

    // lock
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                sc.lock_funds(ManagedAddress::from(&claimer_addr));
            },
        )
        .assert_ok();

    b_mock
        .execute_tx(&user_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.withdraw();
        })
        .assert_error(4, "caller has nothing to claim");

    b_mock
        .execute_tx(&claimer_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.withdraw();
        })
        .assert_user_error("requested funds are still locked");

    // unlock ok
    b_mock.set_block_epoch(10);

    b_mock
        .execute_tx(&claimer_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.withdraw();
        })
        .assert_ok();
    b_mock.check_esdt_balance(&claimer_addr, LOCKED_TOKEN_ID, &lock_amount);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                sc.lock_funds(ManagedAddress::from(&claimer_addr));
            },
        )
        .assert_user_error("caller cannot use this contract at this time");

    // lock with same token, same unlock epoch -> same token nonce
    b_mock.set_block_epoch(15);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                sc.lock_funds(ManagedAddress::from(&claimer_addr));
            },
        )
        .assert_ok();
}
