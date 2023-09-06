#![allow(deprecated)]

mod energy_factory_setup;

use energy_factory::{
    energy::{Energy, EnergyModule},
    token_merging::TokenMergingModule,
    token_whitelist::TokenWhitelistModule,
    SimpleLockEnergy,
};
use energy_factory_setup::*;
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{
        BigInt, BigUint, EgldOrEsdtTokenIdentifier, EgldOrEsdtTokenPayment, EsdtLocalRole,
        MultiValueEncoded,
    },
};
use multiversx_sc_modules::pause::PauseModule;
use multiversx_sc_scenario::{
    managed_address, managed_token_id, whitebox_legacy::BlockchainStateWrapper,
    whitebox_legacy::TxTokenTransfer,
};
use simple_lock::{
    basic_lock_unlock::BasicLockUnlock,
    locked_token::{LockedTokenAttributes, LockedTokenModule},
};

use multiversx_sc_scenario::{managed_token_id_wrapped, rust_biguint, DebugApi};

#[test]
fn token_merging_test() {
    let _ = DebugApi::dummy();
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();

    let first_token_amount = 400_000;
    let first_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[0]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            first_token_amount,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let second_token_amount = 100_000;
    let second_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[1]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            second_token_amount,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(400_000),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(100_000),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.sc_wrapper, &payments[..], |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    assert_eq!(first_token_unlock_epoch, 360);
    assert_eq!(second_token_unlock_epoch, 720);

    // (400_000 * 360 + 100_000 * 720) / 500_000 = epoch 432
    // -> start of month (upper) = 450
    let expected_merged_token_unlock_epoch = 450;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(first_token_amount + second_token_amount),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_merged_token_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(500_000) * expected_merged_token_unlock_epoch;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_energy, actual_energy);
}

#[test]
fn token_merging_different_years_test() {
    let _ = DebugApi::dummy();
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();

    let first_token_amount = 400_000;
    let first_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[1]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            first_token_amount,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let second_token_amount = 100_000;
    let second_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[2]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            second_token_amount,
            LOCK_OPTIONS[2],
        )
        .assert_ok();

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(400_000),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(100_000),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.sc_wrapper, &payments[..], |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    assert_eq!(first_token_unlock_epoch, 720);
    assert_eq!(second_token_unlock_epoch, 1440);

    // (400_000 * 720 + 100_000 * 1440) / 500_000 = epoch 864
    // -> start of month (upper) = 870
    let expected_merged_token_unlock_epoch = 870;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(first_token_amount + second_token_amount),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_merged_token_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(500_000) * expected_merged_token_unlock_epoch;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_energy, actual_energy);
}

#[test]
fn token_merging_different_years2_test() {
    let _ = DebugApi::dummy();
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();

    let first_token_amount = 400_000;
    let first_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[0]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            first_token_amount,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let second_token_amount = 100_000;
    let second_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[2]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            second_token_amount,
            LOCK_OPTIONS[2],
        )
        .assert_ok();

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(400_000),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(100_000),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.sc_wrapper, &payments[..], |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    assert_eq!(first_token_unlock_epoch, 360);
    assert_eq!(second_token_unlock_epoch, 1440);

    // (400_000 * 360 + 100_000 * 1_440) / 500_000 = 576 unlock epoch
    // -> start of month (upper) = 600
    let expected_merged_token_unlock_epoch = 600;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(first_token_amount + second_token_amount),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_merged_token_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(500_000) * expected_merged_token_unlock_epoch;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_energy, actual_energy);
}

#[test]
fn test_specific_tokens_merge() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let user = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        energy_factory::contract_obj,
        "energy factory",
    );
    let unbond_sc_mock = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        unbond_sc_mock::UnbondScMock::new,
        "fees collector mock",
    );

    let first_balance = "3562537017212685308192738"
        .parse::<num_bigint::BigUint>()
        .unwrap();
    let total_expected = "5675109497292997578670612"
        .parse::<num_bigint::BigUint>()
        .unwrap();
    let second_balance = &total_expected - &first_balance;

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        BASE_ASSET_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LEGACY_LOCKED_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    b_mock
        .execute_tx(&owner, &sc_wrapper, &rust_zero, |sc| {
            let mut lock_options = MultiValueEncoded::new();
            lock_options.push((365u64, 1_000u64).into());
            lock_options.push((730u64, 5_000u64).into());
            lock_options.push((1_460u64, 8_000u64).into());

            sc.init(
                managed_token_id!(BASE_ASSET_TOKEN_ID),
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                managed_address!(unbond_sc_mock.address_ref()),
                0,
                lock_options,
            );

            sc.base_asset_token_id()
                .set(&managed_token_id!(BASE_ASSET_TOKEN_ID));
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.set_paused(false);

            let _ = sc.lock_and_send(
                &managed_address!(&user),
                EgldOrEsdtTokenPayment::new(
                    EgldOrEsdtTokenIdentifier::esdt(managed_token_id!(BASE_ASSET_TOKEN_ID)),
                    0,
                    BigUint::from_bytes_be(&first_balance.to_bytes_be()),
                ),
                4_140,
            );
            let _ = sc.lock_and_send(
                &managed_address!(&user),
                EgldOrEsdtTokenPayment::new(
                    EgldOrEsdtTokenIdentifier::esdt(managed_token_id!(BASE_ASSET_TOKEN_ID)),
                    0,
                    BigUint::from_bytes_be(&second_balance.to_bytes_be()),
                ),
                4_050,
            );

            sc.user_energy(&managed_address!(&user)).set(Energy::new(
                BigInt::zero(),
                2_695,
                BigUint::from_bytes_be(&total_expected.clone().to_bytes_be()),
            ));
        })
        .assert_ok();

    b_mock.set_block_epoch(2_695);

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: first_balance.clone(),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: second_balance.clone(),
        },
    ];
    b_mock
        .execute_esdt_multi_transfer(&user, &sc_wrapper, &payments, |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    b_mock.check_nft_balance(
        &user,
        LOCKED_TOKEN_ID,
        3,
        &total_expected,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 4_110,
        }),
    );
}

#[test]
fn merge_same_schedule_test() {
    let _ = DebugApi::dummy();
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let user = setup.first_user.clone();
    let unlock_epoch = to_start_of_month(LOCK_OPTIONS[0]);

    let first_token_amount = 400_000;
    setup
        .lock(
            &user,
            BASE_ASSET_TOKEN_ID,
            first_token_amount,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let second_token_amount = 100_000;
    setup
        .lock(
            &user,
            BASE_ASSET_TOKEN_ID,
            second_token_amount,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(400_000),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(100_000),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&user, &setup.sc_wrapper, &payments[..], |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(first_token_amount + second_token_amount),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch,
        }),
    );
}
