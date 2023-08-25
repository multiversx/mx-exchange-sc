#![allow(deprecated)]

mod proxy_dex_test_setup;

use energy_factory::{energy::EnergyModule, SimpleLockEnergy};
use energy_query::Energy;
use multiversx_sc::{
    codec::{multi_types::OptionalValue, Empty},
    types::{BigInt, EsdtTokenPayment},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use num_traits::ToPrimitive;
use proxy_dex::{
    proxy_pair::ProxyPairModule, wrapped_lp_attributes::WrappedLpTokenAttributes,
    wrapped_lp_token_merge::WrappedLpTokenMerge,
};
use proxy_dex_test_setup::*;
use simple_lock::locked_token::LockedTokenAttributes;

#[test]
fn setup_test() {
    let _ = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
}

#[test]
fn add_remove_liquidity_proxy_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let full_balance = rust_biguint!(USER_BALANCE);
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount),
        None,
    );
    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount),
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        1,
        &expected_lp_token_amount,
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
        }),
    );

    // check proxy balance
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &expected_lp_token_amount,
    );

    // check pair balance
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &locked_token_amount,
    );
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &other_token_amount,
    );

    let mut block_epoch = 1u64;
    let mut user_balance = USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // pass epochs to process energy update
    block_epoch = 10u64;
    setup.b_mock.set_block_epoch(block_epoch);
    user_balance -= 1000; // extra_locked_tokens burnt

    // remove liquidity
    let half_lp_tokens = &expected_lp_token_amount / 2u32;
    // should be 500_000_000, but ends up so due to approximations
    let removed_locked_token_amount = rust_biguint!(499_999_000);
    // should be 250_000_000, but ends up so due to approximations
    let removed_other_token_amount = rust_biguint!(249_999_500);
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &half_lp_tokens,
            |sc| {
                let output_payments = sc.remove_liquidity_proxy(
                    managed_address!(&pair_addr),
                    managed_biguint!(1),
                    managed_biguint!(1),
                );
                let output_vec = output_payments.to_vec();

                assert_eq!(output_payments.len(), 2);
                assert_eq!(
                    output_vec.get(0).amount.to_u64().unwrap(),
                    removed_locked_token_amount.to_u64().unwrap()
                );
                assert_eq!(
                    output_vec.get(1).amount.to_u64().unwrap(),
                    removed_other_token_amount.to_u64().unwrap()
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount + &removed_locked_token_amount),
        None,
    );
    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount + &removed_other_token_amount),
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        1,
        &(&expected_lp_token_amount - &half_lp_tokens),
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
        }),
    );
}

#[test]
fn tripple_add_liquidity_proxy_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let full_balance = rust_biguint!(USER_BALANCE);
    let locked_token_amount1 = rust_biguint!(1_000_000_000);
    let locked_token_amount2 = rust_biguint!(1_100_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let other_token_amount2 = rust_biguint!(600_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);
    let expected_second_lp_token_amount = rust_biguint!(500_000_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount1.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // First add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount1.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // check proxy's LOCKED balance
    setup.b_mock.check_nft_balance::<Empty>(
        setup.proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &locked_token_amount1,
        None,
    );

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount1),
        None,
    );
    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount),
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        1,
        &expected_lp_token_amount,
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount1.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
        }),
    );

    // check proxy balance
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &expected_lp_token_amount,
    );

    // check pair balance
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &locked_token_amount1,
    );
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &other_token_amount,
    );

    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount2,
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // Second add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount1.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // check proxy's LOCKED balance
    setup.b_mock.check_nft_balance::<Empty>(
        setup.proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(&locked_token_amount1 * 2u64),
        None,
    );

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount1 * 2u64),
        None,
    );

    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount * 2u64),
    );

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        2,
        &expected_second_lp_token_amount,
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount1.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_second_lp_token_amount.to_u64().unwrap()),
        }),
    );

    // check proxy balance
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &(expected_lp_token_amount.clone() + expected_second_lp_token_amount.clone()),
    );

    // check pair balance
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &(locked_token_amount1.clone() * 2u64),
    );
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &(other_token_amount.clone() * 2u64),
    );

    // Third add liquidity
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount1.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount2,
        },
    ];

    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount1.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // check proxy's LOCKED balance
    setup.b_mock.check_nft_balance::<Empty>(
        setup.proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(&locked_token_amount1 * 3u64),
        None,
    );

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount1 * 3u64),
        None,
    );

    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount * 3u64),
    );

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        3,
        &expected_second_lp_token_amount,
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount1.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_second_lp_token_amount.to_u64().unwrap()),
        }),
    );

    // check proxy balance
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &(expected_lp_token_amount + (expected_second_lp_token_amount * 2u64)),
    );

    // check pair balance
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &(locked_token_amount1 * 3u64),
    );
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &(other_token_amount * 3u64),
    );
}

#[test]
fn wrapped_same_nonce_lp_token_merge_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // total available: 499_999_000
    let first_amount = rust_biguint!(150_000_000);
    let second_amount = rust_biguint!(250_000_000);
    let tokens_to_merge = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: first_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: second_amount.clone(),
        },
    ];

    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &tokens_to_merge, |sc| {
            sc.merge_wrapped_lp_tokens_endpoint();
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        2,
        &rust_biguint!(400_000_000),
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(800_001_600), // out of 1_000_000_000
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(400_000_000),
        }),
    );

    let liquidity_token_amount = (&first_amount + &second_amount) * rust_biguint!(2u64); // parity 1 EGLD -> 2 MEX
    let expected_locked_token_balance_before = rust_biguint!(USER_BALANCE) - locked_token_amount;
    let expected_locked_token_balance_after =
        &expected_locked_token_balance_before + &liquidity_token_amount;

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(expected_locked_token_balance_before),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(first_amount + second_amount),
            |sc| {
                sc.remove_liquidity_proxy(
                    managed_address!(&pair_addr),
                    managed_biguint!(1),
                    managed_biguint!(1),
                );
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &expected_locked_token_balance_after,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );
}

#[test]
fn wrapped_different_nonce_lp_token_merge_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let user = setup.first_user.clone();
    let user_balance = rust_biguint!(USER_BALANCE);
    setup
        .b_mock
        .set_esdt_balance(&user, MEX_TOKEN_ID, &user_balance);

    // users lock tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE));
            },
        )
        .assert_ok();

    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);

    // set the price to 1 EGLD = 2 MEX
    let payments1 = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(), // for MIN_LP_AMOUNT
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(), // for MIN_LP_AMOUNT
        },
    ];

    // add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&user, &setup.proxy_wrapper, &payments1, |sc| {
            let output_lp_token = sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );

            assert_eq!(output_lp_token.to_vec().get(0).token_nonce, 1);
            assert_eq!(
                output_lp_token.to_vec().get(0).amount,
                managed_biguint!(500_000_000u64 - 1_000u64)
            );
        })
        .assert_ok();

    // set the price to 1 EGLD = 2 MEX
    let payments2 = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&user, &setup.proxy_wrapper, &payments2, |sc| {
            let output_lp_token = sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
            assert_eq!(output_lp_token.to_vec().get(0).token_nonce, 2);
            assert_eq!(
                output_lp_token.to_vec().get(0).amount,
                managed_biguint!(500_000_000u64)
            );
        })
        .assert_ok();

    let min_lp_amount = 1_000u64;
    let first_amount = other_token_amount.clone() - rust_biguint!(min_lp_amount);
    let second_amount = other_token_amount.clone();
    let user_lp_amount = 999_999_000u64; // first_amount + second_amount;
    let tokens_to_merge = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: first_amount,
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_LP_TOKEN_ID.to_vec(),
            nonce: 2,
            value: second_amount,
        },
    ];

    setup
        .b_mock
        .execute_esdt_multi_transfer(&user, &setup.proxy_wrapper, &tokens_to_merge, |sc| {
            let output_lp_token = sc.merge_wrapped_lp_tokens_endpoint();
            assert_eq!(output_lp_token.token_nonce, 3);
            assert_eq!(output_lp_token.amount, managed_biguint!(user_lp_amount));
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user,
        WRAPPED_LP_TOKEN_ID,
        3,
        &rust_biguint!(user_lp_amount),
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(2_000_000_000u64),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(1_000_000_000u64 - min_lp_amount),
        }),
    );

    let min_locked_lp_token_amount = rust_biguint!(min_lp_amount);
    let expected_locked_token_balance =
        &(locked_token_amount - min_locked_lp_token_amount) * rust_biguint!(2); // parity 1 EGLD -> 2 MEX

    setup
        .b_mock
        .execute_esdt_transfer(
            &user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            3,
            &rust_biguint!(user_lp_amount),
            |sc| {
                sc.remove_liquidity_proxy(
                    managed_address!(&pair_addr),
                    managed_biguint!(1),
                    managed_biguint!(1),
                );
            },
        )
        .assert_ok();

    let expected_unlock_epoch = (LOCK_OPTIONS[0] + LOCK_OPTIONS[1]) / 2;
    setup.b_mock.check_nft_balance(
        &user,
        LOCKED_TOKEN_ID,
        3,
        &expected_locked_token_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_unlock_epoch,
        }),
    );
}
