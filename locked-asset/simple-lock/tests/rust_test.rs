#![allow(deprecated)]

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::types::EsdtLocalRole;
use multiversx_sc_scenario::{managed_biguint, managed_token_id_wrapped, whitebox_legacy::*};
use multiversx_sc_scenario::{managed_token_id, rust_biguint, DebugApi};

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use simple_lock::locked_token::*;
use simple_lock::SimpleLock;

const FREE_TOKEN_ID: &[u8] = b"FREEEEE-123456";
const LOCKED_TOKEN_ID: &[u8] = b"NOOO0-123456";

#[test]
fn lock_unlock_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        simple_lock::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    let lock_amount = rust_biguint!(1_000);
    b_mock.set_esdt_balance(&user_addr, FREE_TOKEN_ID, &lock_amount);

    // lock
    let mut lock_token_nonce = 0;
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FREE_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                let payment_result = sc.lock_tokens_endpoint(10, OptionalValue::None);
                lock_token_nonce = payment_result.token_nonce;
            },
        )
        .assert_ok();

    // needed for the managed types in LockedTokenAttributes
    let _ = DebugApi::dummy();
    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(FREE_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );

    // try unlock too early
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            lock_token_nonce,
            &lock_amount,
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_user_error("Cannot unlock yet");

    // unlock ok
    b_mock.set_block_epoch(10);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            lock_token_nonce,
            &lock_amount,
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();
    b_mock.check_esdt_balance(&user_addr, FREE_TOKEN_ID, &lock_amount);

    // lock with same token, same unlock epoch -> same token nonce
    b_mock.set_block_epoch(9);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FREE_TOKEN_ID,
            0,
            &rust_biguint!(100),
            |sc| {
                let payment_result = sc.lock_tokens_endpoint(10, OptionalValue::None);
                assert_eq!(payment_result.token_nonce, lock_token_nonce);
            },
        )
        .assert_ok();

    // lock with same token, different unlock epoch -> different attributes -> different nonce
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FREE_TOKEN_ID,
            0,
            &rust_biguint!(100),
            |sc| {
                let payment_result = sc.lock_tokens_endpoint(15, OptionalValue::None);
                assert_eq!(payment_result.token_nonce, lock_token_nonce + 1);
            },
        )
        .assert_ok();

    // test auto-unlock
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FREE_TOKEN_ID,
            0,
            &rust_biguint!(100),
            |sc| {
                let payment_result = sc.lock_tokens_endpoint(5, OptionalValue::None);
                assert_eq!(
                    payment_result.token_identifier,
                    managed_token_id!(FREE_TOKEN_ID)
                );
                assert_eq!(payment_result.token_nonce, 0);
                assert_eq!(payment_result.amount, managed_biguint!(100));
            },
        )
        .assert_ok();
}
