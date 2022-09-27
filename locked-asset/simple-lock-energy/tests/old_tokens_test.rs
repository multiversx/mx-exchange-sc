mod simple_lock_energy_setup;

use common_structs::{LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockScheduleEx};
use elrond_wasm::types::{BigInt, ManagedVec};
use simple_lock::locked_token::LockedTokenModule;
use simple_lock_energy::{
    energy::{Energy, EnergyModule},
    old_token_nonces::OldTokenNonces,
};
use simple_lock_energy_setup::*;

use elrond_wasm_debug::{managed_address, managed_biguint, rust_biguint, DebugApi};

#[test]
fn extend_lock_period_old_token_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);

    setup.b_mock.set_block_epoch(1);

    let first_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR);
    let second_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR * 5);
    let mut unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 40_000,
        unlock_epoch: first_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 60_000,
        unlock_epoch: second_unlock_epoch,
    });
    let old_token_attributes = LockedAssetTokenAttributesEx {
        is_merged: false,
        unlock_schedule: UnlockScheduleEx { unlock_milestones },
    };

    let first_user = setup.first_user.clone();
    setup.b_mock.set_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );
    setup.b_mock.set_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount += managed_biguint!(40_000) * USER_BALANCE * first_unlock_epoch / 100_000u32;
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * second_unlock_epoch / 100_000u32;
    user_energy_amount += managed_biguint!(40_000) * USER_BALANCE * first_unlock_epoch / 100_000u32;
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * second_unlock_epoch / 100_000u32;

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            let _ = sc.old_token_nonces().insert(1u64);
            let _ = sc.old_token_nonces().insert(2u64);

            // create two tokens to update the nonce
            let _ = sc
                .locked_token()
                .nft_create(managed_biguint!(1), &old_token_attributes);
            let _ = sc
                .locked_token()
                .nft_create(managed_biguint!(1), &old_token_attributes);

            sc.user_energy(&managed_address!(&first_user))
                .set(&Energy::new(
                    BigInt::from(user_energy_amount.clone()),
                    1,
                    managed_biguint!(USER_BALANCE) * 2u32,
                ));
        })
        .assert_ok();

    // extend to 5 years
    setup
        .extend_locking_period(&first_user, 1, USER_BALANCE, EPOCHS_IN_YEAR * 5)
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedAssetTokenAttributesEx {
            is_merged: false,
            unlock_schedule: UnlockScheduleEx::<DebugApi> {
                unlock_milestones: ManagedVec::from_single_item(UnlockMilestoneEx {
                    unlock_epoch: second_unlock_epoch,
                    unlock_percent: 100_000,
                }),
            },
        }),
    );

    let energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (second_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(
        to_rust_biguint(user_energy_amount.clone()),
        actual_energy_after
    );

    // try "extend" from 5 to 1 year
    setup
        .extend_locking_period(&first_user, 3, USER_BALANCE, EPOCHS_IN_YEAR)
        .assert_user_error("All unlock periods already exceed the requested period");

    // extend second token to 10 years
    setup
        .extend_locking_period(&first_user, 2, USER_BALANCE, EPOCHS_IN_YEAR * 10)
        .assert_ok();

    let new_unlock = to_start_of_month(EPOCHS_IN_YEAR * 10);
    let first_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock - first_unlock_epoch) / 100_000u32;
    let second_increase =
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock - second_unlock_epoch) / 100_000u32;
    user_energy_amount += first_increase;
    user_energy_amount += second_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(
        to_rust_biguint(user_energy_amount.clone()),
        actual_energy_after
    );
}

#[test]
fn unlock_old_token_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();

    setup.b_mock.set_block_epoch(1);

    setup.b_mock.set_esdt_balance(
        setup.sc_wrapper.address_ref(),
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );
    setup
        .b_mock
        .set_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &rust_zero);

    let first_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR);
    let second_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR * 5);
    let mut unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 40_000,
        unlock_epoch: first_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 60_000,
        unlock_epoch: second_unlock_epoch,
    });
    let mut old_token_attributes = LockedAssetTokenAttributesEx {
        is_merged: false,
        unlock_schedule: UnlockScheduleEx { unlock_milestones },
    };

    setup.b_mock.set_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount += managed_biguint!(40_000) * USER_BALANCE * first_unlock_epoch / 100_000u32;
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * second_unlock_epoch / 100_000u32;

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            let _ = sc.old_token_nonces().insert(1u64);

            // create token to update the nonce
            let _ = sc
                .locked_token()
                .nft_create(managed_biguint!(1), &old_token_attributes);

            sc.user_energy(&managed_address!(&first_user))
                .set(&Energy::new(
                    BigInt::from(user_energy_amount.clone()),
                    1,
                    managed_biguint!(USER_BALANCE),
                ));
        })
        .assert_ok();

    // first_unlock_epoch - 1 epochs pass
    setup.b_mock.set_block_epoch(first_unlock_epoch);
    user_energy_amount -=
        managed_biguint!(40_000) * USER_BALANCE * (first_unlock_epoch - 1) / 100_000u32;
    user_energy_amount -=
        managed_biguint!(60_000) * USER_BALANCE * (first_unlock_epoch - 1) / 100_000u32;

    let expected_unlocked_balance = rust_biguint!(40_000) * USER_BALANCE / 100_000u32;
    let expected_locked_balance = rust_biguint!(60_000) * USER_BALANCE / 100_000u32;
    setup.unlock(&first_user, 1, USER_BALANCE).assert_ok();

    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_unlocked_balance);

    old_token_attributes
        .unlock_schedule
        .unlock_milestones
        .remove(0);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &expected_locked_balance,
        Some(&old_token_attributes),
    );

    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, to_rust_biguint(user_energy_amount));
}
