#![allow(deprecated)]

mod proxy_dex_test_setup;

use energy_factory::SimpleLockEnergy;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc_scenario::{managed_biguint, managed_buffer, managed_token_id, rust_biguint};
use proxy_dex_test_setup::*;
use proxy_dex_xmex::create_pair_user::{CreatePairUserModule, ISSUE_COST};

#[test]
fn create_pair_test() {
    let mut setup = ProxySetup::new(
        proxy_dex_xmex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        router::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let full_balance = rust_biguint!(USER_BALANCE);
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    setup
        .b_mock
        .set_egld_balance(&first_user, &rust_biguint!(ISSUE_COST));

    // owner lock
    let owner = setup.owner.clone();
    setup
        .b_mock
        .set_esdt_balance(&owner, MEX_TOKEN_ID, &full_balance);
    setup
        .b_mock
        .execute_esdt_transfer(
            &owner,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &full_balance,
            |sc| {
                sc.lock_tokens_endpoint(LOCK_OPTIONS[2], OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WEGLD_TOKEN_ID,
            0,
            &other_token_amount,
            |sc| {
                sc.deposit_project_token(managed_biguint!(1));
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_tx(
            &first_user,
            &setup.proxy_wrapper,
            &rust_biguint!(ISSUE_COST),
            |sc| {
                sc.create_xmex_token_pair(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_buffer!(b"Name"),
                    managed_buffer!(b"NAME"),
                );
            },
        )
        .assert_ok();
}
