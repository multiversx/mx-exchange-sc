#![allow(deprecated)]

pub mod metabonding_staking_setup;
use metabonding_staking::{
    locked_asset_token::{LockedAssetTokenModule, UserEntry},
    UNBOND_EPOCHS,
};
use metabonding_staking_setup::*;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, rust_biguint, whitebox_legacy::TxTokenTransfer,
};

#[test]
fn test_init() {
    let _ = MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
}

#[test]
fn test_stake_first() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    setup.call_stake_locked_asset(3, 100_000_000).assert_ok();

    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(3, managed_biguint!(100_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();
}

#[test]
fn test_stake_second() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    setup.call_stake_locked_asset(3, 100_000_000).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_supply = managed_biguint!(100_000_000);
            let actual_supply = sc.total_locked_asset_supply().get();
            assert_eq!(actual_supply, expected_supply);
        })
        .assert_ok();

    setup.call_stake_locked_asset(4, 1_000_000).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_supply = managed_biguint!(101_000_000);
            let actual_supply = sc.total_locked_asset_supply().get();
            assert_eq!(actual_supply, expected_supply);
        })
        .assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();
}

#[test]
fn test_stake_multiple() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_ASSET_TOKEN_ID.to_vec(),
            nonce: 3,
            value: rust_biguint!(100_000_000),
        },
        TxTokenTransfer {
            token_identifier: LOCKED_ASSET_TOKEN_ID.to_vec(),
            nonce: 4,
            value: rust_biguint!(1_000_000),
        },
    ];

    setup
        .call_stake_locked_asset_multiple(&payments)
        .assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();
}

#[test]
fn test_unstake() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    setup.call_stake_locked_asset(3, 100_000_000).assert_ok();
    setup.call_stake_locked_asset(4, 1_000_000).assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    setup.call_unstake(101_000_000).assert_ok();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 1,
                stake_amount: managed_biguint!(0),
                unstake_amount: managed_biguint!(101_000_000),
                unbond_epoch: 3,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // try unstake again
    setup
        .call_unstake(101_000_000)
        .assert_user_error("Trying to unstake too much");
}

#[test]
fn test_partial_unstake() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    setup.call_stake_locked_asset(3, 90_000_000).assert_ok();
    setup.call_stake_locked_asset(4, 1_000_000).assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(1, managed_biguint!(91_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    setup.call_unstake(51_000_000).assert_ok();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 1,
                stake_amount: managed_biguint!(40_000_000),
                unstake_amount: managed_biguint!(51_000_000),
                unbond_epoch: 3,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // unstake too much
    setup
        .call_unstake(101_000_000)
        .assert_user_error("Trying to unstake too much");

    // unstake ok
    setup.b_mock.set_block_epoch(5);
    setup.call_unstake(30_000_000).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 1,
                stake_amount: managed_biguint!(10_000_000),
                unstake_amount: managed_biguint!(81_000_000),
                unbond_epoch: 8,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // stake after unstake
    setup.call_stake_locked_asset(3, 10_000_000).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 2,
                stake_amount: managed_biguint!(20_000_000),
                unstake_amount: managed_biguint!(81_000_000),
                unbond_epoch: 8,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // unbond

    setup.b_mock.set_block_epoch(15);
    setup.call_unbond().assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 2,
                stake_amount: managed_biguint!(20_000_000),
                unstake_amount: managed_biguint!(0),
                unbond_epoch: u64::MAX,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // checking attributes for LKMEX tokens is out of scope
    // so we just check with the raw expected value
    let attributes: Vec<u8> = vec![
        0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 38, 173, 0, 0, 0, 0, 0, 0, 1, 104, 0,
        0, 0, 0, 0, 0, 58, 162, 0, 0, 0, 0, 0, 0, 1, 134, 0, 0, 0, 0, 0, 0, 58, 162, 0, 0, 0, 0, 0,
        0, 1, 164, 0, 0, 0, 0, 0, 0, 58, 171, 0, 0, 0, 0, 0, 0, 1, 194, 0, 0, 0, 0, 0, 0, 58, 172,
        0, 0, 0, 0, 0, 0, 1, 224, 0, 0, 0, 0, 0, 0, 58, 172, 0, 0, 0, 0, 0, 0, 1, 254, 0, 0, 0, 0,
        0, 0, 58, 172, 1,
    ];
    setup.b_mock.check_nft_balance(
        &setup.user_address,
        LOCKED_ASSET_TOKEN_ID,
        2,
        &rust_biguint!(81_000_000),
        Some(&attributes),
    );
}

#[test]
fn test_unbond() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);

    setup.call_stake_locked_asset(3, 100_000_000).assert_ok();
    setup.call_stake_locked_asset(4, 1_000_000).assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // try unbond before unstake
    setup.call_unbond().assert_user_error("Must unstake first");

    setup.call_unstake(101_000_000).assert_ok();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = UserEntry {
                token_nonce: 1,
                stake_amount: managed_biguint!(0),
                unstake_amount: managed_biguint!(101_000_000),
                unbond_epoch: 3,
            };
            let actual_entry = sc.entry_for_user(&managed_address!(&user_addr)).get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_supply = managed_biguint!(101_000_000);
            let actual_supply = sc.total_locked_asset_supply().get();
            assert_eq!(actual_supply, expected_supply);
        })
        .assert_ok();

    // try unbond too early
    setup
        .call_unbond()
        .assert_user_error("Unbond period in progress");

    setup.b_mock.set_block_epoch(UNBOND_EPOCHS);
    setup.call_unbond().assert_ok();

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_supply = managed_biguint!(0);
            let actual_supply = sc.total_locked_asset_supply().get();
            assert_eq!(actual_supply, expected_supply);
        })
        .assert_ok();

    // checking attributes for LKMEX tokens is out of scope
    // so we just check with the raw expected value
    let attributes: Vec<u8> = vec![
        0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 38, 173, 0, 0, 0, 0, 0, 0, 1, 104, 0,
        0, 0, 0, 0, 0, 58, 162, 0, 0, 0, 0, 0, 0, 1, 134, 0, 0, 0, 0, 0, 0, 58, 162, 0, 0, 0, 0, 0,
        0, 1, 164, 0, 0, 0, 0, 0, 0, 58, 171, 0, 0, 0, 0, 0, 0, 1, 194, 0, 0, 0, 0, 0, 0, 58, 172,
        0, 0, 0, 0, 0, 0, 1, 224, 0, 0, 0, 0, 0, 0, 58, 172, 0, 0, 0, 0, 0, 0, 1, 254, 0, 0, 0, 0,
        0, 0, 58, 172, 1,
    ];
    setup.b_mock.check_nft_balance(
        &setup.user_address,
        LOCKED_ASSET_TOKEN_ID,
        1,
        &rust_biguint!(101_000_000),
        Some(&attributes),
    );

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let entry_is_empty = sc.entry_for_user(&managed_address!(&user_addr)).is_empty();
            assert!(entry_is_empty);
        })
        .assert_ok();
}
