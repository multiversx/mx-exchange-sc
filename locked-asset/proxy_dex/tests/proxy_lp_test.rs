mod proxy_dex_test_setup;

use elrond_wasm::{elrond_codec::Empty, types::EsdtTokenPayment};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, tx_mock::TxInputESDT,
    DebugApi,
};
use num_traits::ToPrimitive;
use proxy_dex::{
    proxy_pair::ProxyPairModule, wrapped_lp_attributes::WrappedLpTokenAttributes,
    wrapped_lp_token_merge::WrappedLpTokenMerge,
};
use proxy_dex_test_setup::*;

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
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxInputESDT {
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
fn double_add_liquidity_proxy_test() {
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
    let expected_second_lp_token_amount = rust_biguint!(500_000_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxInputESDT {
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
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    // check proxy's LOCKED balance
    setup.b_mock.check_nft_balance::<Empty>(
        &setup.proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &locked_token_amount,
        None,
    );

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

    // Second add liquidity
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

    // check proxy's LOCKED balance
    setup.b_mock.check_nft_balance::<Empty>(
        &setup.proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(&locked_token_amount * 2u64),
        None,
    );

    // check user's balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount * 2u64),
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
                amount: managed_biguint!(locked_token_amount.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_second_lp_token_amount.to_u64().unwrap()),
        }),
    );

    // check proxy balance
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &(expected_lp_token_amount + expected_second_lp_token_amount),
    );

    // check pair balance
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &(locked_token_amount * 2u64),
    );
    setup.b_mock.check_esdt_balance(
        setup.pair_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &(other_token_amount * 2u64),
    );
}

#[test]
fn wrapped_lp_token_merge_test() {
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
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxInputESDT {
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
        TxInputESDT {
            token_identifier: WRAPPED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: first_amount.clone(),
        },
        TxInputESDT {
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
                token_nonce: 3,
                amount: managed_biguint!(800_001_600), // out of 1_000_000_000
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(400_000_000),
        }),
    );
}
