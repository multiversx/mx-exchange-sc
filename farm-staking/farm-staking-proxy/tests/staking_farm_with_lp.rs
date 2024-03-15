#![allow(deprecated)]

pub mod constants;
pub mod staking_farm_with_lp_external_contracts;
pub mod staking_farm_with_lp_staking_contract_interactions;
pub mod staking_farm_with_lp_staking_contract_setup;

multiversx_sc::imports!();

use constants::*;
use farm_boosted_yields::FarmBoostedYieldsModule;
use farm_staking_proxy::dual_yield_token::DualYieldTokenAttributes;

use farm_staking_proxy::proxy_actions::unstake::ProxyUnstakeModule;

use multiversx_sc::codec::Empty;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, DebugApi,
};
use staking_farm_with_lp_staking_contract_interactions::*;

#[test]
fn test_all_setup() {
    let _ = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );
}

#[test]
fn test_stake_farm_proxy() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000; // safe price of USER_TOTAL_LP_TOKENS in RIDE tokens
    let _dual_yield_token_nonce =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);
}

#[test]
fn test_claim_rewards_farm_proxy_full() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);

    let dual_yield_token_amount = expected_staking_token_amount;
    let _dual_yield_token_nonce_after_claim = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        99_999,
        1_899,
        dual_yield_token_amount,
    );
}

#[test]
fn test_claim_rewards_farm_proxy_half() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);

    let dual_yield_token_amount = expected_staking_token_amount / 2;
    let _dual_yield_token_nonce_after_claim = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        99_999 / 2,
        949,
        dual_yield_token_amount,
    );
}

#[test]
fn test_claim_rewards_farm_proxy_twice() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    // first claim, at block 120
    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);

    let dual_yield_token_amount = expected_staking_token_amount;
    let dual_yield_token_nonce_after_first_claim = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        99_999,
        1_899,
        dual_yield_token_amount,
    );

    // second claim, at block 140
    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 40);

    let dual_yield_token_amount = expected_staking_token_amount;
    let _ = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_first_claim,
        dual_yield_token_amount,
        99_999,
        1_899,
        dual_yield_token_amount,
    );
}

#[test]
fn test_unstake_through_proxy_no_claim() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    let dual_yield_token_amount = 1_001_000_000;
    setup.unstake_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        1_001_000_000,
        99_999,
        1_899,
        1_001_000_000,
        30,
    );
}

#[test]
fn unstake_through_proxy_after_claim() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    let dual_yield_token_amount = expected_staking_token_amount;
    let dual_yield_token_nonce_after_claim = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        99_999,
        1_899,
        dual_yield_token_amount,
    );

    let dual_yield_token_amount = 1_001_000_000;
    setup.unstake_proxy(
        dual_yield_token_nonce_after_claim,
        dual_yield_token_amount,
        1_001_000_000,
        0,
        0,
        1_001_000_000,
        30,
    );
}

#[test]
fn unstake_partial_position_test() {
    DebugApi::dummy();
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    let dual_yield_token_amount = 1_001_000_000;

    // unstake with half position - ok
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            dual_yield_token_nonce_after_stake,
            &rust_biguint!(dual_yield_token_amount / 2),
            |sc| {
                let results = sc.unstake_farm_tokens(
                    managed_biguint!(1),
                    managed_biguint!(1),
                    OptionalValue::None,
                );

                let wegld_payment = results.other_token_payment;
                assert_eq!(
                    wegld_payment.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(wegld_payment.amount, dual_yield_token_amount / 2);

                let lp_farm_rewards = results.lp_farm_rewards;
                assert_eq!(
                    lp_farm_rewards.token_identifier,
                    managed_token_id!(LOCKED_TOKEN_ID)
                );
                assert_eq!(lp_farm_rewards.amount, 99_999 / 2);

                let staking_rewards = results.staking_rewards;
                assert_eq!(
                    staking_rewards.token_identifier,
                    managed_token_id!(RIDE_TOKEN_ID)
                );
                assert_eq!(staking_rewards.amount, 1_899 / 2);

                let unbond_tokens = results.unbond_staking_farm_token;
                assert_eq!(
                    unbond_tokens.token_identifier,
                    managed_token_id!(STAKING_FARM_TOKEN_ID)
                );
                assert_eq!(unbond_tokens.amount, dual_yield_token_amount / 2);
            },
        )
        .assert_ok();

    // unstake with the remaining dual yield tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user_addr,
            &setup.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            dual_yield_token_nonce_after_stake,
            &rust_biguint!(dual_yield_token_amount / 2),
            |sc| {
                let results = sc.unstake_farm_tokens(
                    managed_biguint!(1),
                    managed_biguint!(1),
                    OptionalValue::None,
                );

                let wegld_payment = results.other_token_payment;
                assert_eq!(
                    wegld_payment.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(wegld_payment.amount, 1_001_000_000 / 2);

                let lp_farm_rewards = results.lp_farm_rewards;
                assert_eq!(
                    lp_farm_rewards.token_identifier,
                    managed_token_id!(LOCKED_TOKEN_ID)
                );
                assert_eq!(lp_farm_rewards.amount, 99_999 / 2);

                let staking_rewards = results.staking_rewards;
                assert_eq!(
                    staking_rewards.token_identifier,
                    managed_token_id!(RIDE_TOKEN_ID)
                );
                assert_eq!(staking_rewards.amount, 1_899 / 2);

                let unbond_tokens = results.unbond_staking_farm_token;
                assert_eq!(
                    unbond_tokens.token_identifier,
                    managed_token_id!(STAKING_FARM_TOKEN_ID)
                );
                assert_eq!(unbond_tokens.amount, 1_001_000_000 / 2);
            },
        )
        .assert_ok();
}

#[test]
fn unbond_test() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 1_001_000_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp_proxy(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
    setup.b_mock.set_block_epoch(20);

    let dual_yield_token_amount = expected_staking_token_amount;
    let dual_yield_token_nonce_after_claim = setup.claim_rewards_proxy(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        99_999,
        1_899,
        dual_yield_token_amount,
    );

    let dual_yield_token_amount = 1_001_000_000;
    let unbond_token_nonce = setup.unstake_proxy(
        dual_yield_token_nonce_after_claim,
        dual_yield_token_amount,
        1_001_000_000,
        0,
        0,
        1_001_000_000,
        30,
    );

    setup.b_mock.set_block_epoch(30);

    let unbond_amount = 1_001_000_000;
    setup.unbond_proxy(unbond_token_nonce, unbond_amount, unbond_amount);
}

#[test]
fn farm_staking_compound_rewards_and_unstake_test() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );
    let farming_amount = 500_000_000;

    let mut farm_staking_nonce = setup.stake_farm(farming_amount, farming_amount);

    setup
        .b_mock
        .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 100);
    setup.b_mock.set_block_epoch(10);

    let new_farming_amount = 500_004_700; // 47 * 100, limited by the APR
    farm_staking_nonce =
        setup.staking_farm_compound_rewards(farm_staking_nonce, farming_amount, new_farming_amount);

    let expected_nr_unbond_tokens = new_farming_amount;
    let _ = setup.staking_farm_unstake(
        farm_staking_nonce,
        new_farming_amount,
        0,
        expected_nr_unbond_tokens,
    );
}

#[test]
fn test_stake_farm_through_proxy_with_merging() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let first_dual_yield_token_nonce = setup.stake_farm_lp_proxy(1, 400_000_000, 1, 400_000_000);

    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance(
            &setup.user_addr,
            DUAL_YIELD_TOKEN_ID,
            first_dual_yield_token_nonce,
            &rust_biguint!(400_000_000),
            Some(&DualYieldTokenAttributes::<DebugApi> {
                lp_farm_token_nonce: 1,
                lp_farm_token_amount: managed_biguint!(400_000_000),
                staking_farm_token_nonce: 1,
                staking_farm_token_amount: managed_biguint!(400_000_000),
            }),
        )
    });

    let dual_yield_token_payments = vec![NonceAmountPair {
        nonce: first_dual_yield_token_nonce,
        amount: 400_000_000,
    }];
    let new_dual_yield_token_nonce =
        setup.stake_farm_lp_proxy_multiple(1, 600_000_000, dual_yield_token_payments);

    // check user staking farm tokens
    setup.b_mock.check_nft_balance::<Empty>(
        &setup.user_addr,
        DUAL_YIELD_TOKEN_ID,
        first_dual_yield_token_nonce,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance(
            &setup.user_addr,
            DUAL_YIELD_TOKEN_ID,
            new_dual_yield_token_nonce,
            &rust_biguint!(1_000_000_000),
            Some(&DualYieldTokenAttributes::<DebugApi> {
                lp_farm_token_nonce: 2,
                lp_farm_token_amount: managed_biguint!(1_000_000_000),
                staking_farm_token_nonce: 2,
                staking_farm_token_amount: managed_biguint!(1_000_000_000),
            }),
        )
    });

    // check farm staking SC tokens
    setup.b_mock.check_esdt_balance(
        setup.staking_farm_wrapper.address_ref(),
        RIDE_TOKEN_ID,
        &rust_biguint!(1_000_000_000_000),
    );

    // check proxy SC tokens
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance::<Empty>(
            setup.proxy_wrapper.address_ref(),
            LP_FARM_TOKEN_ID,
            2,
            &rust_biguint!(1_000_000_000),
            None, //current attributes
        )
    });
}

#[test]
fn test_farm_stake_proxy_merging_boosted_rewards() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    // Boosted rewards setup
    setup.set_lp_farm_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    let farm_amount = 50_000_000u64;
    let user_address = setup.user_addr.clone();
    let temp_user = setup
        .b_mock
        .create_user_account(&rust_biguint!(100_000_000));
    setup.exit_lp_farm(&user_address, 1, USER_TOTAL_LP_TOKENS);
    setup.b_mock.set_esdt_balance(
        &setup.user_addr,
        LP_TOKEN_ID,
        &rust_biguint!(farm_amount * 2),
    );
    setup
        .b_mock
        .set_esdt_balance(&temp_user, LP_TOKEN_ID, &rust_biguint!(1));

    setup.b_mock.set_block_epoch(2);

    setup.set_user_energy(&user_address, 1_000, 2, 1);
    let mut farm_token_nonce = setup.enter_lp_farm(&user_address, farm_amount);
    let second_farm_token_nonce = setup.enter_lp_farm(&user_address, farm_amount); // will enter Metastaking next week

    // User claims rewards to get his energy registered
    farm_token_nonce = setup.claim_lp_farm(&user_address, farm_token_nonce, farm_amount, 0);

    // User enters Metastaking
    let first_dual_yield_token_nonce =
        setup.stake_farm_lp_proxy(farm_token_nonce, farm_amount, 1, farm_amount);
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance(
            &setup.user_addr,
            DUAL_YIELD_TOKEN_ID,
            first_dual_yield_token_nonce,
            &rust_biguint!(farm_amount),
            Some(&DualYieldTokenAttributes::<DebugApi> {
                lp_farm_token_nonce: farm_token_nonce,
                lp_farm_token_amount: managed_biguint!(farm_amount),
                staking_farm_token_nonce: 1,
                staking_farm_token_amount: managed_biguint!(farm_amount),
            }),
        )
    });

    // advance blocks - 10 blocks - 10 * 5_000 = 50_000 total rewards
    // 37_500 base farm, 12_500 boosted yields
    let boosted_rewards = 12_500u64;
    setup.b_mock.set_block_nonce(110);

    // random tx on end of week 1, to cummulate rewards
    setup.b_mock.set_block_epoch(6);
    setup.set_user_energy(&user_address, 1_000, 6, 1);
    setup.set_user_energy(&temp_user, 1, 6, 1);
    let temp_user_farm_token_nonce = setup.enter_lp_farm(&temp_user, 1);
    setup.exit_lp_farm(&temp_user, temp_user_farm_token_nonce, 1);

    // advance 1 week
    setup.b_mock.set_block_epoch(10);
    setup.set_user_energy(&user_address, 1_000, 10, 1);

    // check locked tokens rewards before staking farm tokens with merge
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance::<Empty>(
            &user_address,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(0),
            None,
        )
    });

    // user enters Metastaking with second position, which should merge with the first one
    let dual_yield_token_payments = vec![NonceAmountPair {
        nonce: first_dual_yield_token_nonce,
        amount: farm_amount,
    }];
    let new_dual_yield_token_nonce = setup.stake_farm_lp_proxy_multiple(
        second_farm_token_nonce,
        farm_amount,
        dual_yield_token_payments,
    );

    // check user staking farm tokens
    setup.b_mock.check_nft_balance::<Empty>(
        &setup.user_addr,
        DUAL_YIELD_TOKEN_ID,
        first_dual_yield_token_nonce,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance(
            &setup.user_addr,
            DUAL_YIELD_TOKEN_ID,
            new_dual_yield_token_nonce,
            &rust_biguint!(farm_amount * 2),
            Some(&DualYieldTokenAttributes::<DebugApi> {
                lp_farm_token_nonce: 6,
                lp_farm_token_amount: managed_biguint!(farm_amount * 2),
                staking_farm_token_nonce: 2,
                staking_farm_token_amount: managed_biguint!(farm_amount * 2),
            }),
        )
    });

    // check farm staking SC tokens
    setup.b_mock.check_esdt_balance(
        setup.staking_farm_wrapper.address_ref(),
        RIDE_TOKEN_ID,
        &rust_biguint!(1_000_000_000_000),
    );

    // check proxy SC tokens
    setup.b_mock.execute_in_managed_environment(|| {
        setup.b_mock.check_nft_balance::<Empty>(
            setup.proxy_wrapper.address_ref(),
            LP_FARM_TOKEN_ID,
            6, // farm token nonce after merge
            &rust_biguint!(farm_amount * 2),
            None, //current attributes
        )
    });

    setup
        .b_mock
        .execute_query(&setup.lp_farm_wrapper, |sc| {
            let rewards = sc
                .accumulated_rewards_per_user(&managed_address!(&user_address))
                .get();
            assert_eq!(rewards, managed_biguint!(boosted_rewards));
        })
        .assert_ok();
}
