#![allow(deprecated)]

mod tests_common;
use multiversx_sc_scenario::{managed_biguint, rust_biguint};
use price_discovery::user_actions::user_deposit_withdraw::UserDepositWithdrawModule;
use tests_common::*;

#[test]
fn setup_test() {
    let _ = PriceDiscSetup::new(price_discovery::contract_obj);
}

#[test]
fn user_deposit_too_early_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);
    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_user_error("User deposit/withdraw not allowed in this phase");
}

#[test]
fn user_deposit_over_limit_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);
    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.second_user_address.clone(), 11_000)
        .assert_user_error("Exceeded deposit limit");
}

#[test]
fn user_not_in_whitelist_try_deposit_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);
    setup.b_mock.set_block_timestamp(START_TIME + 1);

    let new_user_address = setup.b_mock.create_user_account(&rust_biguint!(0));
    setup
        .b_mock
        .set_esdt_balance(&new_user_address, ACCEPTED_TOKEN_ID, &rust_biguint!(1_000));

    setup
        .call_user_deposit(&new_user_address, 1_000)
        .assert_user_error("User not whitelisted");
}

#[test]
fn user_deposit_too_few_tokens_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);
    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.second_user_address.clone(), 99)
        .assert_user_error("Not enough tokens deposited");
}

#[test]
fn user_deposit_ok_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(1_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 1_000),
    );
    setup
        .b_mock
        .execute_query(&setup.pd_wrapper, |sc| {
            assert_eq!(sc.total_deposit_by_user(1).get(), 1_000);
        })
        .assert_ok();
}

#[test]
fn user_withdraw_too_much_after_deposit_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();

    setup
        .call_user_withdraw(&setup.first_user_address.clone(), 950)
        .assert_user_error("Withdrawing too many tokens");
}

#[test]
fn user_deposit_withdraw_ok_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_withdraw(&setup.first_user_address.clone(), 400)
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(600),
    );
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 600),
    );
    setup
        .b_mock
        .execute_query(&setup.pd_wrapper, |sc| {
            assert_eq!(sc.total_deposit_by_user(1).get(), 600);
        })
        .assert_ok();
}

#[test]
fn owner_deposit_too_early_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_owner_deposit(1_000)
        .assert_user_error("Owner deposit/withdraw not allowed in this phase");
}

#[test]
fn owner_deposit_ok_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();
}

#[test]
fn owner_withdraw_too_much_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();

    setup
        .call_owner_withdraw(1_500)
        .assert_user_error("Invalid amount");
}

#[test]
fn owner_withdraw_ok_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();

    setup.call_owner_withdraw(500).assert_ok();

    setup.b_mock.check_esdt_balance(
        &setup.owner_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 1_500),
    );
    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(1_500),
    );
}

#[test]
fn user_redeem_too_early_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();

    setup
        .call_user_redeem(&setup.first_user_address.clone())
        .assert_user_error("Redeem not allowed in this phase");
}

#[test]
fn user_redeem_no_owner_deposit() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + OWNER_DEPOSIT_TIME + 1);

    setup
        .call_user_redeem(&setup.first_user_address.clone())
        .assert_ok();
    setup
        .call_user_redeem(&setup.second_user_address.clone())
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );
    setup.b_mock.check_esdt_balance(
        &setup.second_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );
}

#[test]
fn user_redeem_ok_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + OWNER_DEPOSIT_TIME + 1);

    setup
        .call_user_redeem(&setup.first_user_address.clone())
        .assert_ok();
    setup
        .call_user_redeem(&setup.second_user_address.clone())
        .assert_ok();
    setup.call_owner_redeem().assert_ok();

    // owner try withdraw twice
    setup
        .call_owner_redeem()
        .assert_error(10, "insufficient funds");

    // check accepted token balance
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 1_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.second_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 9_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.owner_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(10_000),
    );
    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(0),
    );

    // check launched token balance
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(200),
    );
    setup.b_mock.check_esdt_balance(
        &setup.second_user_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(1_800),
    );
    setup.b_mock.check_esdt_balance(
        &setup.owner_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 2_000),
    );
    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(0),
    );
}

#[test]
fn refund_user_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();
    setup
        .call_user_deposit(&setup.second_user_address.clone(), 9_000)
        .assert_ok();

    setup
        .call_refund_user(&setup.first_user_address.clone())
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(9_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + 1);

    setup.call_owner_deposit(2_000).assert_ok();

    setup
        .b_mock
        .set_block_timestamp(START_TIME + USER_DEPOSIT_TIME + OWNER_DEPOSIT_TIME + 1);

    // user try redeem after refunded
    setup
        .call_user_redeem(&setup.first_user_address.clone())
        .assert_user_error("User not whitelisted");

    setup
        .call_user_redeem(&setup.second_user_address.clone())
        .assert_ok();
    setup.call_owner_redeem().assert_ok();

    // check accepted token balance
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );
    setup.b_mock.check_esdt_balance(
        &setup.second_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 9_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.owner_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(9_000),
    );
    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(0),
    );

    // check launched token balance
    setup.b_mock.check_esdt_balance(
        &setup.first_user_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(0),
    );
    setup.b_mock.check_esdt_balance(
        &setup.second_user_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(2_000),
    );
    setup.b_mock.check_esdt_balance(
        &setup.owner_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(USER_BALANCE - 2_000),
    );
    setup.b_mock.check_esdt_balance(
        setup.pd_wrapper.address_ref(),
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(0),
    );
}

#[test]
fn set_user_limit_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 1);

    setup
        .call_user_deposit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();

    // set limit ok
    setup
        .call_set_user_limit(&setup.first_user_address.clone(), 1_500)
        .assert_ok();

    // set limit ok
    setup
        .call_set_user_limit(&setup.first_user_address.clone(), 1_000)
        .assert_ok();

    // set limit value too low
    setup
        .call_set_user_limit(&setup.first_user_address.clone(), 500)
        .assert_user_error("May not set user limit below current deposit value");

    // check limit has the correct value
    setup
        .b_mock
        .execute_query(&setup.pd_wrapper, |sc| {
            assert_eq!(sc.user_deposit_limit(1).get(), managed_biguint!(1_000));
        })
        .assert_ok();
}

#[test]
fn set_timestamp_invalid_values_test() {
    let mut setup = PriceDiscSetup::new(price_discovery::contract_obj);

    setup.b_mock.set_block_timestamp(START_TIME + 50);

    // set timestamp ok
    setup
        .call_set_user_deposit_withdraw_timestamp(START_TIME + 90)
        .assert_ok();

    // set timestamp too low
    setup
        .call_set_user_deposit_withdraw_timestamp(START_TIME + 20)
        .assert_user_error("Invalid timestamp change");
}
