use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm::types::EsdtLocalRole;
use elrond_wasm_debug::testing_framework::*;
use elrond_wasm_debug::{managed_token_id, rust_biguint, DebugApi};

use elrond_wasm::storage::mappers::StorageTokenWrapper;
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
                .set_token_id(&managed_token_id!(LOCKED_TOKEN_ID));
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
                let payment_result = sc.lock_tokens(10, OptionalValue::None);
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
        &LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id!(FREE_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        },
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
                sc.unlock_tokens(OptionalValue::None);
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
                sc.unlock_tokens(OptionalValue::None);
            },
        )
        .assert_ok();
    b_mock.check_esdt_balance(&user_addr, FREE_TOKEN_ID, &lock_amount);
}
