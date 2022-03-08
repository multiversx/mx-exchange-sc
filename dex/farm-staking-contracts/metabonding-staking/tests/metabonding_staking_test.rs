pub mod metabonding_staking_setup;
use elrond_wasm_debug::{managed_address, managed_biguint, rust_biguint, tx_mock::TxInputESDT};
use metabonding_staking::{
    locked_asset_token::{LockedAssetTokenModule, StakingEntry},
    UNBOND_EPOCHS,
};
use metabonding_staking_setup::*;

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
            let expected_entry = StakingEntry::new(3, managed_biguint!(100_000_000));
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();
}

#[test]
fn test_stake_second() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    setup.call_stake_locked_asset(3, 100_000_000).assert_ok();
    setup.call_stake_locked_asset(4, 1_000_000).assert_ok();

    // tokens are merged into a single one
    let user_addr = setup.user_address.clone();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = StakingEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();
}

#[test]
fn test_stake_multiple() {
    let mut setup =
        MetabondingStakingSetup::new(metabonding_staking::contract_obj, factory::contract_obj);
    let payments = [
        TxInputESDT {
            token_identifier: LOCKED_ASSET_TOKEN_ID.to_vec(),
            nonce: 3,
            value: rust_biguint!(100_000_000),
        },
        TxInputESDT {
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
            let expected_entry = StakingEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
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
            let expected_entry = StakingEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    setup.call_unstake().assert_ok();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = StakingEntry {
                nonce: 1,
                amount: managed_biguint!(101_000_000),
                opt_unbond_epoch: Option::Some(10),
            };
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // try unstake again
    setup.call_unstake().assert_user_error("Already unstaked");
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
            let expected_entry = StakingEntry::new(1, managed_biguint!(101_000_000));
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // try unbond before unstake
    setup.call_unbond().assert_user_error("Must unstake first");

    setup.call_unstake().assert_ok();
    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let expected_entry = StakingEntry {
                nonce: 1,
                amount: managed_biguint!(101_000_000),
                opt_unbond_epoch: Option::Some(10),
            };
            let actual_entry = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .get();
            assert_eq!(actual_entry, expected_entry);
        })
        .assert_ok();

    // try unbond too early
    setup
        .call_unbond()
        .assert_user_error("Unbond period in progress");

    setup.b_mock.set_block_epoch(UNBOND_EPOCHS);
    setup.call_unbond().assert_ok();

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
        &attributes,
    );

    setup
        .b_mock
        .execute_query(&setup.mbs_wrapper, |sc| {
            let entry_is_empty = sc
                .staking_entry_for_user(&managed_address!(&user_addr))
                .is_empty();
            assert_eq!(entry_is_empty, true);
        })
        .assert_ok();
}
