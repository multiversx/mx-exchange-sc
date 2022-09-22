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

    let mut current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

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

    let first_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[0]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: first_unlock_epoch,
        }),
    );

    let mut expected_user_energy =
        rust_biguint!(half_balance) * (first_unlock_epoch - current_epoch);
    let mut actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // check energy after half a year
    let half_year_epochs = EPOCHS_IN_YEAR / 2;
    current_epoch += half_year_epochs;
    setup.b_mock.set_block_epoch(current_epoch);

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

    let second_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[0]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: second_unlock_epoch,
        }),
    );

    expected_user_energy += rust_biguint!(half_balance) * (second_unlock_epoch - current_epoch);
    actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // try unlock before deadline
    setup
        .unlock(&first_user, 1, half_balance)
        .assert_user_error("Cannot unlock yet");

    // unlock first tokens
    current_epoch = 1 + LOCK_OPTIONS[0];
    setup.b_mock.set_block_epoch(current_epoch);

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

    let mut current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // unlock early after half a year - with half a year remaining
    // unlock epoch = 360, so epochs remaining after half year (1 + 365 / 2 = 183)
    // = 360 - 183 = 177
    let half_year_epochs = EPOCHS_IN_YEAR / 2;
    current_epoch += half_year_epochs;
    setup.b_mock.set_block_epoch(current_epoch);

    let penalty_percentage = 485u64; // 1 + 9_999 * 177 / (10 * 365) ~= 1 + 484 = 485
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, 177);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup.unlock_early(&first_user, 1, half_balance).assert_ok();

    let received_token_amount = rust_biguint!(half_balance) - penalty_amount;
    let expected_balance = received_token_amount + half_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);

    let expected_energy = rust_biguint!(0);
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);
}

#[test]
fn reduce_lock_period_test() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // reduce half year worth of epochs, 180 ~= 6 months
    let half_year_epochs = 180;

    let penalty_percentage = 494u64; // 1 + 9_999 * 180 / (10 * 365) ~= 1 + 493 = 494
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

    let expected_locked_token_balance = rust_biguint!(half_balance) - &penalty_amount;
    let expected_new_unlock_epoch = to_start_of_month(360 - half_year_epochs);
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

    // check the tokens were half burned, half set to fees collector
    setup.b_mock.check_esdt_balance(
        &setup.sc_wrapper.address_ref(),
        BASE_ASSET_TOKEN_ID,
        &expected_locked_token_balance,
    );
    setup.b_mock.check_esdt_balance(
        &setup.fees_collector_mock,
        BASE_ASSET_TOKEN_ID,
        &(penalty_amount / 2u64),
    );

    // check new energy amount
    let expected_energy =
        rust_biguint!(expected_new_unlock_epoch - current_epoch) * expected_locked_token_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);
}

#[test]
fn extend_locking_period_test() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // extend to 3 years - unsupported option
    setup
        .extend_locking_period(&first_user, 1, half_balance, 3 * EPOCHS_IN_YEAR)
        .assert_user_error("Invalid lock choice");

    // extend to 5 years
    setup
        .extend_locking_period(&first_user, 1, half_balance, LOCK_OPTIONS[1])
        .assert_ok();

    let new_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[1]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(new_unlock_epoch - current_epoch) * half_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);

    // try "extend" to 1 year
    setup
        .extend_locking_period(&first_user, 2, half_balance, LOCK_OPTIONS[0])
        .assert_user_error("New lock period must be longer than the current one.");
}

#[test]
fn test_same_token_nonce() {
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let mut current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // lock again after 10 epochs
    current_epoch += 10;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );
}
