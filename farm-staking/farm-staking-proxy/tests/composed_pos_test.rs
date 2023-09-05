#![allow(deprecated)]

pub mod constants;
pub mod staking_farm_with_lp_external_contracts;
pub mod staking_farm_with_lp_staking_contract_interactions;
pub mod staking_farm_with_lp_staking_contract_setup;

multiversx_sc::imports!();

use constants::*;
use farm_staking::stake_farm::StakeFarmModule;
use farm_staking_proxy::dual_yield_token::DualYieldTokenAttributes;
use farm_staking_proxy::proxy_actions::claim::ProxyClaimModule;
use farm_staking_proxy::proxy_actions::merge_pos::ProxyMergePosModule;

use farm_staking_proxy::proxy_actions::unstake::ProxyUnstakeModule;
use multiversx_sc::codec::Empty;
use multiversx_sc_scenario::{
    managed_biguint, rust_biguint, testing_framework::TxTokenTransfer, DebugApi,
};
use staking_farm_with_lp_staking_contract_interactions::*;

#[test]
fn combine_metastaking_with_staking_pos_test() {
    let _ = DebugApi::dummy();
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000; // safe price of USER_TOTAL_LP_TOKENS in RIDE tokens
    let _ = setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    let user_addr = setup.user_addr.clone();
    let composed_pos_full_amount = 1_000_000;
    setup.b_mock.set_esdt_balance(
        &user_addr,
        STAKING_TOKEN_ID,
        &rust_biguint!(composed_pos_full_amount),
    );

    // proxy needs the burn role now the tokens
    setup.b_mock.set_esdt_local_roles(
        setup.proxy_wrapper.address_ref(),
        STAKING_FARM_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    // stake 1/4 and then 3/4 of the tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.staking_farm_wrapper,
            STAKING_TOKEN_ID,
            0,
            &rust_biguint!(composed_pos_full_amount / 4),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.staking_farm_wrapper,
            STAKING_TOKEN_ID,
            0,
            &rust_biguint!(composed_pos_full_amount * 3 / 4),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance::<Empty>(
        &user_addr,
        STAKING_FARM_TOKEN_ID,
        2,
        &rust_biguint!(composed_pos_full_amount / 4),
        None,
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &user_addr,
        STAKING_FARM_TOKEN_ID,
        3,
        &rust_biguint!(composed_pos_full_amount * 3 / 4),
        None,
    );

    // merge metastaking pos with two staking tokens
    let merge_payments = vec![
        TxTokenTransfer {
            token_identifier: DUAL_YIELD_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(expected_staking_token_amount),
        },
        TxTokenTransfer {
            token_identifier: STAKING_FARM_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(composed_pos_full_amount / 4),
        },
        TxTokenTransfer {
            token_identifier: STAKING_FARM_TOKEN_ID.to_vec(),
            nonce: 3,
            value: rust_biguint!(composed_pos_full_amount * 3 / 4),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&user_addr, &setup.proxy_wrapper, &merge_payments, |sc| {
            sc.merge_metastaking_with_staking_token();
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        2,
        &rust_biguint!(expected_staking_token_amount),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 2,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS),
            virtual_pos_token_nonce: 6,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount),
        }),
    );

    // claim rewards with composed pos
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            2,
            &rust_biguint!(expected_staking_token_amount),
            |sc| {
                sc.claim_dual_yield_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        3,
        &rust_biguint!(expected_staking_token_amount),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 3,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS),
            virtual_pos_token_nonce: 7,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount),
        }),
    );

    // claim again
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            3,
            &rust_biguint!(expected_staking_token_amount),
            |sc| {
                sc.claim_dual_yield_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        4,
        &rust_biguint!(expected_staking_token_amount),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 4,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS),
            virtual_pos_token_nonce: 8,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount),
        }),
    );

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    // unstake
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            4,
            &rust_biguint!(expected_staking_token_amount),
            |sc| {
                let unstake_result = sc.unstake_farm_tokens(
                    managed_biguint!(1),
                    managed_biguint!(1),
                    managed_biguint!(expected_staking_token_amount),
                    OptionalValue::None,
                );
                assert_eq!(
                    unstake_result
                        .opt_unbond_staking_farm_token_for_user_pos
                        .unwrap()
                        .amount,
                    composed_pos_full_amount
                );
                assert_eq!(
                    unstake_result.unbond_staking_farm_token.amount,
                    expected_staking_token_amount
                );
            },
        )
        .assert_ok();
}

#[test]
fn combine_metastaking_with_staking_pos_partial_actions_test() {
    let _ = DebugApi::dummy();
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000; // safe price of USER_TOTAL_LP_TOKENS in RIDE tokens
    let _ = setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    let user_addr = setup.user_addr.clone();
    let composed_pos_full_amount = 1_000_000;
    setup.b_mock.set_esdt_balance(
        &user_addr,
        STAKING_TOKEN_ID,
        &rust_biguint!(composed_pos_full_amount),
    );

    // proxy needs the burn role now the tokens
    setup.b_mock.set_esdt_local_roles(
        setup.proxy_wrapper.address_ref(),
        STAKING_FARM_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    // stake 1/4 and then 3/4 of the tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.staking_farm_wrapper,
            STAKING_TOKEN_ID,
            0,
            &rust_biguint!(composed_pos_full_amount / 4),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.staking_farm_wrapper,
            STAKING_TOKEN_ID,
            0,
            &rust_biguint!(composed_pos_full_amount * 3 / 4),
            |sc| {
                sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance::<Empty>(
        &user_addr,
        STAKING_FARM_TOKEN_ID,
        2,
        &rust_biguint!(composed_pos_full_amount / 4),
        None,
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &user_addr,
        STAKING_FARM_TOKEN_ID,
        3,
        &rust_biguint!(composed_pos_full_amount * 3 / 4),
        None,
    );

    // merge half of metastaking pos with two staking tokens
    let merge_payments = vec![
        TxTokenTransfer {
            token_identifier: DUAL_YIELD_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(expected_staking_token_amount / 2),
        },
        TxTokenTransfer {
            token_identifier: STAKING_FARM_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(composed_pos_full_amount / 4),
        },
        TxTokenTransfer {
            token_identifier: STAKING_FARM_TOKEN_ID.to_vec(),
            nonce: 3,
            value: rust_biguint!(composed_pos_full_amount * 3 / 4),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&user_addr, &setup.proxy_wrapper, &merge_payments, |sc| {
            sc.merge_metastaking_with_staking_token();
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        2,
        &rust_biguint!(expected_staking_token_amount / 2),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 2,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS / 2),
            virtual_pos_token_nonce: 6,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount / 2),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount),
        }),
    );

    // claim rewards with part of composed pos
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            2,
            &rust_biguint!(expected_staking_token_amount / 4),
            |sc| {
                sc.claim_dual_yield_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        3,
        &rust_biguint!(expected_staking_token_amount / 4),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 3,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS / 4),
            virtual_pos_token_nonce: 7,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount / 4),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount / 2),
        }),
    );

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    // unstake partial
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            3,
            &rust_biguint!(expected_staking_token_amount / 4),
            |sc| {
                let unstake_result = sc.unstake_farm_tokens(
                    managed_biguint!(1),
                    managed_biguint!(1),
                    managed_biguint!(expected_staking_token_amount / 8),
                    OptionalValue::None,
                );
                assert_eq!(
                    unstake_result
                        .opt_unbond_staking_farm_token_for_user_pos
                        .unwrap()
                        .amount,
                    composed_pos_full_amount / 4
                );
                assert_eq!(
                    unstake_result.unbond_staking_farm_token.amount,
                    expected_staking_token_amount / 8
                );
                assert_eq!(
                    unstake_result.opt_new_dual_yield_tokens.unwrap().amount,
                    expected_staking_token_amount / 8
                );
            },
        )
        .assert_ok();

    // check leftover token attributes
    setup.b_mock.check_nft_balance(
        &user_addr,
        DUAL_YIELD_TOKEN_ID,
        4,
        &rust_biguint!(expected_staking_token_amount / 8),
        Some(&DualYieldTokenAttributes::<DebugApi> {
            lp_farm_token_nonce: 3,
            lp_farm_token_amount: managed_biguint!(USER_TOTAL_LP_TOKENS / 8),
            virtual_pos_token_nonce: 7,
            virtual_pos_token_amount: managed_biguint!(expected_staking_token_amount / 8),
            real_pos_token_amount: managed_biguint!(composed_pos_full_amount / 4),
        }),
    );

    // unstake remaining pos
    setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            4,
            &rust_biguint!(expected_staking_token_amount / 8),
            |sc| {
                let unstake_result = sc.unstake_farm_tokens(
                    managed_biguint!(1),
                    managed_biguint!(1),
                    managed_biguint!(expected_staking_token_amount / 8),
                    OptionalValue::None,
                );
                assert_eq!(
                    unstake_result
                        .opt_unbond_staking_farm_token_for_user_pos
                        .unwrap()
                        .amount,
                    composed_pos_full_amount / 4
                );
                assert_eq!(
                    unstake_result.unbond_staking_farm_token.amount,
                    expected_staking_token_amount / 8
                );
                assert!(unstake_result.opt_new_dual_yield_tokens.is_none(),);
            },
        )
        .assert_ok();
}
