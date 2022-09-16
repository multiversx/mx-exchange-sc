mod simple_lock_energy_setup;

use simple_lock::locked_token::LockedTokenAttributes;
use simple_lock_energy_setup::*;

use elrond_wasm_debug::{managed_token_id_wrapped, rust_biguint, DebugApi};

#[test]
fn init_test() {
    let _ = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
}

#[test]
fn try_lock() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    setup
        .b_mock
        .set_esdt_balance(&first_user, b"FAKETOKEN-123456", &rust_biguint!(1_000));

    // wrong token
    setup
        .lock(&first_user, b"FAKETOKEN-123456", 1_000, LOCK_OPTIONS[0])
        .assert_user_error("May only lock the whitelisted token");

    // invalid lock option
    setup
        .lock(&first_user, BASE_ASSET_TOKEN_ID, USER_BALANCE, 42)
        .assert_user_error("Invalid lock choice");
}

#[test]
fn lock_ok() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    setup.b_mock.set_block_epoch(1);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 1 + LOCK_OPTIONS[0],
        }),
    );

    let mut expected_user_energy = rust_biguint!(half_balance) * LOCK_OPTIONS[0];
    let mut actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // check energy after half a year
    let half_year_epochs = EPOCHS_IN_YEAR / 2;
    setup.b_mock.set_block_epoch(1 + half_year_epochs);

    expected_user_energy -= rust_biguint!(half_balance) * half_year_epochs;
    actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // lock more tokens
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &rust_biguint!(0));
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 1 + LOCK_OPTIONS[0],
        }),
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 1 + half_year_epochs + LOCK_OPTIONS[0],
        }),
    );

    expected_user_energy += rust_biguint!(half_balance) * (LOCK_OPTIONS[0]);
    actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // try unlock before deadline
    setup
        .unlock(&first_user, 1, half_balance)
        .assert_user_error("Cannot unlock yet");

    // unlock first tokens
    setup.b_mock.set_block_epoch(1 + LOCK_OPTIONS[0]);

    setup.unlock(&first_user, 1, half_balance).assert_ok();
    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );
}

#[test]
fn unlock_early_test() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    setup.b_mock.set_block_epoch(1);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // unlock early after half a year - with half a year remaining
    let half_year_epochs = EPOCHS_IN_YEAR / 2;
    setup.b_mock.set_block_epoch(1 + half_year_epochs + 1);

    let penalty_percentage = 499u64; // 1 + 9_999 * 182 / (10 * 365) ~= 1 + 9_999 / 20
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, half_year_epochs);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup.unlock_early(&first_user, 1, half_balance).assert_ok();

    let received_token_amount = rust_biguint!(half_balance) - penalty_amount;
    let expected_balance = received_token_amount + half_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);
}

#[test]
fn reduce_lock_period_test() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    setup.b_mock.set_block_epoch(1);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // reduce half year worth of epochs
    let half_year_epochs = EPOCHS_IN_YEAR / 2;

    let penalty_percentage = 499u64; // 1 + 9_999 * 182 / (10 * 365) ~= 1 + 9_999 / 20
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, half_year_epochs);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup
        .reduce_lock_period(&first_user, 1, half_balance, half_year_epochs)
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );

    let expected_locked_token_balance = rust_biguint!(half_balance) - penalty_amount;
    let expected_new_unlock_epoch = 1 + LOCK_OPTIONS[0] - half_year_epochs;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &expected_locked_token_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_new_unlock_epoch,
        }),
    );
}
