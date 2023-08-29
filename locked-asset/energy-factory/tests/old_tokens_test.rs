#![allow(deprecated)]

mod energy_factory_setup;

use common_structs::{
    LockedAssetTokenAttributes, LockedAssetTokenAttributesEx, UnlockMilestone, UnlockMilestoneEx,
    UnlockSchedule, UnlockScheduleEx,
};
use energy_factory::{
    energy::{Energy, EnergyModule},
    migration::SimpleLockMigrationModule,
};
use energy_factory_setup::*;
use multiversx_sc::types::{BigInt, ManagedVec, MultiValueEncoded};
use multiversx_sc_modules::pause::PauseModule;
use simple_lock::locked_token::LockedTokenAttributes;

use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, rust_biguint, DebugApi,
};

#[test]
fn extend_lock_period_old_token_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    let first_unlock_epoch = 91; // 3 months
    let second_unlock_epoch = 121; // 9 months
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
        LEGACY_LOCKED_TOKEN_ID,
        FIRST_UPDATED_BLOCK_NONCE,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * (first_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(60_000) * USER_BALANCE * (second_unlock_epoch - current_epoch)
            / 100_000u32;

    let user_energy_amount_vec = user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            sc.set_paused(true);
            let mut users_energy = MultiValueEncoded::new();
            let user_energy = (
                managed_address!(&first_user),
                managed_biguint!(USER_BALANCE),
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
            )
                .into();
            users_energy.push(user_energy);
            sc.set_energy_for_old_tokens(users_energy);

            let expected_energy = Energy::new(
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
                1,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);

            sc.set_paused(false);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            FIRST_UPDATED_BLOCK_NONCE,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let _ = sc.migrate_old_tokens();
            },
        )
        .assert_ok();

    // (40% * x * 90 + 60% * x * 120) / x = 36 + 72 = 108
    let new_lock_epochs: multiversx_sc::types::BigUint<DebugApi> =
        (managed_biguint!(40_000) * USER_BALANCE / 100_000u32
            * (first_unlock_epoch - current_epoch)
            + managed_biguint!(60_000) * USER_BALANCE / 100_000u32
                * (second_unlock_epoch - current_epoch))
            / USER_BALANCE;
    // rounded up to next month -> 1 + 432 = to_start_month(433) => 420 => 450
    let new_unlock_epoch =
        to_start_of_month(current_epoch + new_lock_epochs.to_u64().unwrap() * 4) + 30;
    assert_eq!(new_unlock_epoch, 450);

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let mut energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    energy_increase +=
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock_epoch - second_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(to_rust_biguint(user_energy_amount), actual_energy_after);
}

#[test]
fn min_period_migrated_token_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    let first_unlock_epoch = 91; // 3 months
    let second_unlock_epoch = 121; // 9 months
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
        LEGACY_LOCKED_TOKEN_ID,
        FIRST_UPDATED_BLOCK_NONCE,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * (first_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(60_000) * USER_BALANCE * (second_unlock_epoch - current_epoch)
            / 100_000u32;

    let user_energy_amount_vec = user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            sc.set_paused(true);
            let mut users_energy = MultiValueEncoded::new();
            let user_energy = (
                managed_address!(&first_user),
                managed_biguint!(USER_BALANCE),
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
            )
                .into();
            users_energy.push(user_energy);
            sc.set_energy_for_old_tokens(users_energy);

            let expected_energy = Energy::new(
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
                1,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);

            sc.set_paused(false);

            sc.min_migrated_token_locked_period().set(1_000);
        })
        .assert_ok();

    let new_unlock_epoch = 1_020u64; // estimated to nearest month (upwards)
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            FIRST_UPDATED_BLOCK_NONCE,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let _ = sc.migrate_old_tokens();
            },
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
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let mut energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    energy_increase +=
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock_epoch - second_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(to_rust_biguint(user_energy_amount), actual_energy_after);
}

#[test]
fn min_period_migrated_token_test2() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);

    let current_epoch = 1_441;
    setup.b_mock.set_block_epoch(current_epoch);

    let first_unlock_epoch = 1_621;
    let second_unlock_epoch = 1_711;
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
        LEGACY_LOCKED_TOKEN_ID,
        FIRST_UPDATED_BLOCK_NONCE,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * (first_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(60_000) * USER_BALANCE * (second_unlock_epoch - current_epoch)
            / 100_000u32;

    let user_energy_amount_vec = user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            sc.set_paused(true);

            let mut users_energy = MultiValueEncoded::new();
            let user_energy = (
                managed_address!(&first_user),
                managed_biguint!(USER_BALANCE),
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
            )
                .into();
            users_energy.push(user_energy);
            sc.set_energy_for_old_tokens(users_energy);

            let expected_energy = Energy::new(
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
                1441,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);

            sc.set_paused(false);

            sc.min_migrated_token_locked_period().set(720);
        })
        .assert_ok();

    // (40% * x * 180 + 60% * x * 270) / x = 72 + 162 = 234
    let new_lock_epochs: multiversx_sc::types::BigUint<DebugApi> =
        (managed_biguint!(40_000) * USER_BALANCE / 100_000u32
            * (first_unlock_epoch - current_epoch)
            + managed_biguint!(60_000) * USER_BALANCE / 100_000u32
                * (second_unlock_epoch - current_epoch))
            / USER_BALANCE;
    // rounded up to next month -> 1 + 432 = to_start_month(1440 + 234 * 4) => 2376 => 2400
    let new_unlock_epoch =
        to_start_of_month(current_epoch + new_lock_epochs.to_u64().unwrap() * 4) + 30;
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            FIRST_UPDATED_BLOCK_NONCE,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let _ = sc.migrate_old_tokens();
            },
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
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let mut energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    energy_increase +=
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock_epoch - second_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(to_rust_biguint(user_energy_amount), actual_energy_after);
}

#[test]
fn check_initial_old_unlock_schedule_decode_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    let first_unlock_epoch = 91; // 3 months
    let second_unlock_epoch = 121; // 9 months
    let mut unlock_milestones = ManagedVec::<DebugApi, UnlockMilestone>::new();
    unlock_milestones.push(UnlockMilestone {
        unlock_percent: 40u8,
        unlock_epoch: first_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestone {
        unlock_percent: 60u8,
        unlock_epoch: second_unlock_epoch,
    });
    let old_token_attributes = LockedAssetTokenAttributes {
        is_merged: false,
        unlock_schedule: UnlockSchedule { unlock_milestones },
    };

    let first_user = setup.first_user.clone();
    setup.b_mock.set_nft_balance(
        &first_user,
        LEGACY_LOCKED_TOKEN_ID,
        1, // nonce < FIRST_UPDATED_BLOCK_NONCE
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * (first_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(60_000) * USER_BALANCE * (second_unlock_epoch - current_epoch)
            / 100_000u32;

    let user_energy_amount_vec = user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            sc.set_paused(true);
            let mut users_energy = MultiValueEncoded::new();
            let user_energy = (
                managed_address!(&first_user),
                managed_biguint!(USER_BALANCE),
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
            )
                .into();
            users_energy.push(user_energy);
            sc.set_energy_for_old_tokens(users_energy);

            let expected_energy = Energy::new(
                BigInt::from_signed_bytes_be(&user_energy_amount_vec),
                1,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);

            sc.set_paused(false);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.sc_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let _ = sc.migrate_old_tokens();
            },
        )
        .assert_ok();

    // (40% * x * 90 + 60% * x * 120) / x = 36 + 72 = 108
    let new_lock_epochs: multiversx_sc::types::BigUint<DebugApi> =
        (managed_biguint!(40_000) * USER_BALANCE / 100_000u32
            * (first_unlock_epoch - current_epoch)
            + managed_biguint!(60_000) * USER_BALANCE / 100_000u32
                * (second_unlock_epoch - current_epoch))
            / USER_BALANCE;
    // rounded up to next month -> 1 + 432 = to_start_month(433) => 420 => 450
    let new_unlock_epoch =
        to_start_of_month(current_epoch + new_lock_epochs.to_u64().unwrap() * 4) + 30;
    assert_eq!(new_unlock_epoch, 450);

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let mut energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    energy_increase +=
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock_epoch - second_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(to_rust_biguint(user_energy_amount), actual_energy_after);
}
