#![allow(deprecated)]

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::types::{EsdtLocalRole, MultiValueEncoded};
use multiversx_sc_scenario::{managed_token_id, rust_biguint, DebugApi};
use multiversx_sc_scenario::{managed_token_id_wrapped, whitebox_legacy::*};

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use simple_lock::locked_token::*;
use simple_lock_whitelist::SimpleLockWhitelist;

static FREE_TOKEN_ID: &[u8] = b"FREEEEE-123456";
static OTHER_TOKEN_ID: &[u8] = b"ILLEGAL-123456";
static LOCKED_TOKEN_ID: &[u8] = b"NOOO0-123456";

#[test]
fn lock_whitelist_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        simple_lock_whitelist::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            let mut whitelist = MultiValueEncoded::new();
            whitelist.push(managed_token_id!(FREE_TOKEN_ID));

            sc.init(whitelist);
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
    b_mock.set_esdt_balance(&user_addr, OTHER_TOKEN_ID, &lock_amount);

    // lock wrong token
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            OTHER_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                let _ = sc.lock_tokens_endpoint(10, OptionalValue::None);
            },
        )
        .assert_user_error("Invalid payments");

    // lock ok
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FREE_TOKEN_ID,
            0,
            &lock_amount,
            |sc| {
                let _ = sc.lock_tokens_endpoint(10, OptionalValue::None);
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_TOKEN_ID,
        1,
        &lock_amount,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(FREE_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );
}
