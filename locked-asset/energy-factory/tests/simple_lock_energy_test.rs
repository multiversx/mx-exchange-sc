#![allow(deprecated)]

mod energy_factory_setup;

use energy_factory::{
    energy::EnergyModule, locked_token_transfer::LockedTokenTransferModule, SimpleLockEnergy,
};
use energy_factory_setup::*;
use multiversx_sc::types::BigUint;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, Zero};
use simple_lock::locked_token::LockedTokenAttributes;

use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, rust_biguint, DebugApi,
};

#[test]
fn init_test() {
    let _ = SimpleLockEnergySetup::new(energy_factory::contract_obj);
}

#[test]
fn try_lock() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    setup
        .b_mock
        .set_esdt_balance(&first_user, b"FAKETOKEN-123456", &rust_biguint!(1_000));

    // wrong token
    setup
        .lock(&first_user, b"FAKETOKEN-123456", 1_000, LOCK_OPTIONS[0])
        .assert_user_error("Invalid payment token");

    // invalid lock option
    setup
        .lock(&first_user, BASE_ASSET_TOKEN_ID, USER_BALANCE, 42)
        .assert_user_error("Invalid lock choice");
}

#[test]
fn lock_ok() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
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
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let penalty_percentage = 4_000u64; // 1 year = 4_000
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, LOCK_OPTIONS[0], 0);
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
fn multiple_early_unlocks_same_week_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;
    let sixth_balance = half_balance / 3;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let mut penalty_percentage = 4_000u64; // 1 year = 4_000
    let mut expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    let mut penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    // Unlock early 1/3 of the LockedTokens
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    let received_token_amount = rust_biguint!(sixth_balance) - penalty_amount;

    // After first early unlock of the week, fees are sent to the unstake sc
    setup.b_mock.check_nft_balance(
        &setup.unbond_sc_mock,
        LOCKED_TOKEN_ID,
        1,
        &expected_penalty_amount,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );

    // Unlock early the another 1/3 of the LockedTokens, same week
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    penalty_percentage = 4_000u64; // 1 year = 4_000
    expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    let received_token_amount_2 = rust_biguint!(sixth_balance) - penalty_amount;

    // Unlock early the last 1/3 of the LockedTokens, same week
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    penalty_percentage = 4_000u64; // 1 year = 4_000
    expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    let received_token_amount_3 = rust_biguint!(sixth_balance) - penalty_amount;
    let expected_balance =
        &received_token_amount_3 + &received_token_amount_2 + &received_token_amount + half_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);
}

#[test]
fn reduce_lock_period_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let penalty_percentage = 3_333u64; // (6_000 - 4_000) / (10_000 - 4_000) = 3_333
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, LOCK_OPTIONS[1], LOCK_OPTIONS[0]);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup
        .reduce_lock_period(&first_user, 1, half_balance, LOCK_OPTIONS[0])
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );

    let expected_locked_token_balance = rust_biguint!(half_balance) - &penalty_amount;
    let expected_new_unlock_epoch = EPOCHS_IN_YEAR; // from 2 initial years - 1 year = 1 years
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

    // Fees are sent to unstake SC
    setup.b_mock.check_nft_balance(
        &setup.unbond_sc_mock,
        LOCKED_TOKEN_ID,
        1,
        &penalty_amount,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 720,
        }),
    );

    // check new energy amount
    let expected_energy =
        rust_biguint!(expected_new_unlock_epoch - current_epoch) * expected_locked_token_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);
}

#[test]
fn extend_locking_period_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let random_user = setup.b_mock.create_user_account(&rust_biguint!(0u64));
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

    // extend to a random period - unsupported option
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            half_balance,
            3 * EPOCHS_IN_YEAR,
            None,
        )
        .assert_user_error("Invalid lock choice");

    // extend to a whitelisted period, but for a different user
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            half_balance,
            LOCK_OPTIONS[1],
            Some(random_user),
        )
        .assert_user_error("May not use the optional destination argument here");

    // extend to the second option - should work as intended
    // 1 epoch has passed
    let energy_per_epoch = rust_biguint!(500_000_000_000_000_000u64);
    let energy_before = setup.get_user_energy(&first_user); // 179_500_000_000_000_000_000
    assert_eq!(
        energy_before,
        LOCK_OPTIONS[0] * energy_per_epoch.clone() - energy_per_epoch.clone()
    );
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            half_balance,
            LOCK_OPTIONS[1],
            None,
        )
        .assert_ok();
    let energy_after = setup.get_user_energy(&first_user); // 359_500_000_000_000_000_000
    assert_eq!(
        energy_after,
        LOCK_OPTIONS[1] * energy_per_epoch.clone() - energy_per_epoch.clone()
    );

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
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            2,
            half_balance,
            LOCK_OPTIONS[0],
            None,
        )
        .assert_user_error("New lock period must be longer than the current one");
}

#[test]
fn test_same_token_nonce() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
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

#[test]
fn energy_deplete_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let mut current_epoch = 0;
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * half_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);

    current_epoch = 10;
    let expected_energy: BigUint<DebugApi> =
        managed_biguint!(LOCK_OPTIONS[0] - current_epoch) * half_balance;
    let expected_energy_vec = expected_energy.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_query(&setup.sc_wrapper, |sc| {
            let mut energy = sc.user_energy(&managed_address!(&first_user)).get();
            energy.deplete(current_epoch);
            assert_eq!(
                energy.get_energy_amount(),
                BigUint::from_bytes_be(&expected_energy_vec)
            );
        })
        .assert_ok();
}

#[test]
fn extend_lock_period_endpoint_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            USER_BALANCE,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.extend_lock_period(100, managed_address!(&first_user));
            },
        )
        .assert_user_error("Invalid lock choice");

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.extend_lock_period(LOCK_OPTIONS[0], managed_address!(&first_user));
            },
        )
        .assert_user_error("May not call this endpoint. Use lockTokens instead");

    let energy_per_epoch = rust_biguint!(USER_BALANCE);
    let energy_before = setup.get_user_energy(&first_user);
    assert_eq!(
        energy_before,
        LOCK_OPTIONS[0] * energy_per_epoch.clone() - energy_per_epoch.clone()
    );

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.token_transfer_whitelist()
                    .add(&managed_address!(&first_user));
                sc.extend_lock_period(LOCK_OPTIONS[1], managed_address!(&first_user));
            },
        )
        .assert_ok();

    let energy_after = setup.get_user_energy(&first_user);
    assert_eq!(
        energy_after,
        LOCK_OPTIONS[1] * energy_per_epoch.clone() - energy_per_epoch.clone()
    );
}

#[test]
fn adjust_user_energy_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let user = setup.first_user.clone();
    let user_balance = 1_000_000u64;

    // Initial lock to create user energy
    setup
        .lock(&user, BASE_ASSET_TOKEN_ID, user_balance, LOCK_OPTIONS[0])
        .assert_ok();

    // Check initial energy
    let initial_locked_tokens = rust_biguint!(user_balance);
    let initial_energy = initial_locked_tokens.clone() * LOCK_OPTIONS[0];
    assert_eq!(setup.get_user_locked_tokens(&user), initial_locked_tokens);
    assert_eq!(setup.get_user_energy(&user), initial_energy);

    // Case 1: Positive energy, positive token amount
    let adjustment1_energy = 1_000_000i64;
    let adjustment1_tokens = 500_000i64;
    setup.adjust_user_energy(&user, adjustment1_energy, adjustment1_tokens);

    let locked_tokens1 = setup.get_user_locked_tokens(&user);
    let energy1 = setup.get_user_energy(&user);
    assert_eq!(
        locked_tokens1,
        &initial_locked_tokens + &rust_biguint!(adjustment1_tokens)
    );
    assert_eq!(
        energy1,
        &initial_energy + &rust_biguint!(adjustment1_energy)
    );

    // Case 2: Positive energy, negative token amount
    let adjustment2_energy = 800_000i64;
    let adjustment2_tokens = -300_000i64;
    setup.adjust_user_energy(&user, adjustment2_energy, adjustment2_tokens);

    let locked_tokens2 = setup.get_user_locked_tokens(&user);
    let energy2 = setup.get_user_energy(&user);
    assert_eq!(
        locked_tokens2,
        &locked_tokens1 - &rust_biguint!(adjustment2_tokens.unsigned_abs())
    );
    assert_eq!(energy2, &energy1 + &rust_biguint!(adjustment2_energy));

    // Case 3: Negative energy, positive token amount
    let adjustment3_energy = -1_500_000i64;
    let adjustment3_tokens = 200_000i64;
    setup.adjust_user_energy(&user, adjustment3_energy, adjustment3_tokens);

    let locked_tokens3 = setup.get_user_locked_tokens(&user);
    let energy3 = setup.get_user_energy(&user);
    assert_eq!(
        locked_tokens3,
        &locked_tokens2 + &rust_biguint!(adjustment3_tokens)
    );
    assert_eq!(
        energy3,
        &energy2 - &rust_biguint!(adjustment3_energy.unsigned_abs())
    );

    // Case 4: Negative energy, negative token amount
    let adjustment4_energy = -700_000i64;
    let adjustment4_tokens = -100_000i64;
    setup.adjust_user_energy(&user, adjustment4_energy, adjustment4_tokens);

    let locked_tokens4 = setup.get_user_locked_tokens(&user);
    let energy4 = setup.get_user_energy(&user);
    assert_eq!(
        locked_tokens4,
        &locked_tokens3 - &rust_biguint!(adjustment4_tokens.unsigned_abs())
    );
    assert_eq!(
        energy4,
        &energy3 - &rust_biguint!(adjustment4_energy.unsigned_abs())
    );

    // Bring energy to negative value
    let adjustment5_energy = -500_000_000i64; // Adjust to negative energy
    let adjustment5_tokens = 0i64; // Positive token amount
    setup.adjust_user_energy(&user, adjustment5_energy, adjustment5_tokens);

    let locked_tokens5 = setup.get_user_locked_tokens(&user);
    let energy5 = setup.get_user_energy_raw(&user);
    assert_eq!(
        locked_tokens5,
        &locked_tokens4 + &rust_biguint!(adjustment5_tokens)
    );
    assert!(energy5 < BigInt::zero()); // Energy should be negative

    // Replay Case 1 on negative energy
    let adjustment6_energy = 500_000i64;
    let adjustment6_tokens = 200_000i64;
    setup.adjust_user_energy(&user, adjustment6_energy, adjustment6_tokens);

    let locked_tokens6 = setup.get_user_locked_tokens(&user);
    let energy6 = setup.get_user_energy_raw(&user);
    assert_eq!(
        locked_tokens6,
        &locked_tokens5 + &rust_biguint!(adjustment6_tokens)
    );
    assert_eq!(
        energy6,
        &energy5 + &BigInt::from_i64(adjustment6_energy).unwrap()
    );
    assert!(energy6 < BigInt::zero()); // Energy should remain negative

    // Replay Case 2 on negative energy
    let adjustment7_energy = 300_000i64;
    let adjustment7_tokens = -100_000i64;
    setup.adjust_user_energy(&user, adjustment7_energy, adjustment7_tokens);

    let locked_tokens7 = setup.get_user_locked_tokens(&user);
    let energy7 = setup.get_user_energy_raw(&user);
    assert_eq!(
        locked_tokens7,
        &locked_tokens6 - &rust_biguint!(adjustment7_tokens.unsigned_abs())
    );
    assert_eq!(
        energy7,
        &energy6 + &BigInt::from_i64(adjustment7_energy).unwrap()
    );
    assert!(energy7 < BigInt::zero()); // Energy should remain negative

    // Replay Case 3 on negative energy
    let adjustment8_energy = -500_000i64;
    let adjustment8_tokens = 100_000i64;
    setup.adjust_user_energy(&user, adjustment8_energy, adjustment8_tokens);

    let locked_tokens8 = setup.get_user_locked_tokens(&user);
    let energy8 = setup.get_user_energy_raw(&user);
    assert_eq!(
        locked_tokens8,
        &locked_tokens7 + &rust_biguint!(adjustment8_tokens)
    );
    assert_eq!(
        energy8,
        &energy7 + &BigInt::from_i64(adjustment8_energy).unwrap()
    );
    assert!(energy8 < BigInt::zero()); // Energy should remain negative

    // Replay Case 4 on negative energy
    let adjustment9_energy = -300_000i64;
    let adjustment9_tokens = -50_000i64;
    setup.adjust_user_energy(&user, adjustment9_energy, adjustment9_tokens);

    let locked_tokens9 = setup.get_user_locked_tokens(&user);
    let energy9 = setup.get_user_energy_raw(&user);
    assert_eq!(
        locked_tokens9,
        &locked_tokens8 - &rust_biguint!(adjustment9_tokens.unsigned_abs())
    );
    assert_eq!(
        energy9,
        &energy8 + &BigInt::from_i64(adjustment9_energy).unwrap()
    );
    assert!(energy9 < BigInt::zero()); // Energy should remain negative
}
