#![allow(deprecated)]

pub mod farm_staking_setup;
use config::ConfigModule;
use farm_boosted_yields::undistributed_rewards::UndistributedRewardsModule;
use farm_staking::{
    claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule,
    claim_stake_farm_rewards::ClaimStakeFarmRewardsModule,
    stake_farm::StakeFarmModule,
    token_attributes::{StakingFarmTokenAttributes, UnbondSftAttributes},
    unstake_farm::UnstakeFarmModule,
    FarmStaking,
};
use farm_staking_setup::*;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, rust_biguint, testing_framework::TxTokenTransfer, DebugApi,
};

#[test]
fn farm_staking_with_energy_setup_test() {
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
}

#[test]
fn farm_staking_boosted_rewards_no_energy_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    fs_setup.stake_farm(
        &user_address,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    fs_setup.check_farm_token_supply(farm_in_amount);

    fs_setup.set_block_epoch(5);
    fs_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    // originally, it was 40, but since 25% of the rewards go to boosted yields
    // rewards are now only 3/4 * 40 = 30
    let expected_reward_token_out = 30;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let expected_reward_per_share = 300_000; // from 400_000 -> 300_000
    fs_setup.claim_rewards(
        &user_address,
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
    );
    fs_setup.check_farm_token_supply(farm_in_amount);
}

#[test]
fn farm_staking_other_user_enter_negative_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let farm_in_amount = 100_000_000;
    fs_setup
        .stake_farm_for_other_user(&rand_user, &user_address, farm_in_amount, &[])
        .assert_error(4, "Item not whitelisted");

    let expected_farm_token_nonce = 1;
    fs_setup.stake_farm(
        &user_address,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );

    fs_setup
        .claim_farm_for_other_user(
            &rand_user,
            &user_address,
            expected_farm_token_nonce,
            farm_in_amount,
        )
        .assert_error(4, "Item not whitelisted");

    fs_setup
        .unstake_farm_for_other_user(
            &rand_user,
            &user_address,
            expected_farm_token_nonce,
            farm_in_amount,
        )
        .assert_error(4, "Item not whitelisted");
}

#[test]
fn farm_staking_boosted_rewards_with_energy_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();
    let user_address2 = fs_setup.user_address2.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&user_address, 9_800, 0, 100);
    fs_setup.set_user_energy(&user_address2, 4_900, 0, 350);

    let farm_in_amount = 100_000_000;
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&user_address2, farm_in_amount, &[], 2, 0, 0);
    fs_setup.check_farm_token_supply(farm_in_amount * 2);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address2,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards - week 1
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 6, 1);
    fs_setup.set_block_epoch(6);
    fs_setup.set_block_nonce(10);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            5,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards - week 2
    fs_setup.set_user_energy(&rand_user, 1, 13, 1);
    fs_setup.set_block_epoch(13);
    fs_setup.set_block_nonce(20);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards - week 3
    fs_setup.set_user_energy(&rand_user, 1, 20, 1);
    fs_setup.set_block_epoch(20);
    fs_setup.set_block_nonce(30);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            9,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards - week 4
    fs_setup.set_user_energy(&rand_user, 1, 27, 1);
    fs_setup.set_block_epoch(27);
    fs_setup.set_block_nonce(40);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            11,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(28);
    fs_setup.update_energy_for_user(&user_address);
    fs_setup.update_energy_for_user(&user_address2);

    let base_rewards = 136;
    let boosted_rewards_user = 61;
    let boosted_rewards_user2 = 15; // ~ 1/4 rewards than user1 (half the energy for only 2 weeks)
    let expected_reward_token_out_user = base_rewards + boosted_rewards_user;
    let expected_reward_token_out_user2 = base_rewards + boosted_rewards_user2;
    let expected_farming_token_balance_user =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out_user);
    let expected_farming_token_balance_user2 =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out_user2);
    let expected_reward_per_share = 1_360_000;
    fs_setup.claim_rewards(
        &user_address,
        farm_in_amount,
        3,
        expected_reward_token_out_user,
        &expected_farming_token_balance_user,
        &expected_farming_token_balance_user,
        13,
        expected_reward_per_share,
    );
    fs_setup.claim_rewards(
        &user_address2,
        farm_in_amount,
        4,
        expected_reward_token_out_user2,
        &expected_farming_token_balance_user2,
        &expected_farming_token_balance_user2,
        14,
        expected_reward_per_share,
    );
    fs_setup.check_farm_token_supply(farm_in_amount * 2);
}

#[test]
fn farm_staking_partial_position_handling_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&user_address, 10_000, 0, 10);

    let farm_in_amount = 100_000_000;
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 1, 0, 0);
    fs_setup.check_farm_token_supply(farm_in_amount);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_nonce(10);

    // random user tx to collect rewards

    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 5, 1);
    fs_setup.set_block_epoch(5);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(8);

    fs_setup.set_user_energy(&user_address, 10_000, 8, 10);

    let full_position_base_rewards = 30;
    let boosted_rewards_user = 10;
    let half_position_expected_rewards = full_position_base_rewards / 2 + boosted_rewards_user;
    let expected_farming_token_balance_user =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + half_position_expected_rewards);

    fs_setup.unstake_farm(
        &user_address,
        farm_in_amount / 2,
        2,
        half_position_expected_rewards,
        &expected_farming_token_balance_user,
        &expected_farming_token_balance_user,
        5,
        farm_in_amount / 2,
        &UnbondSftAttributes {
            unlock_epoch: 8 + MIN_UNBOND_EPOCHS,
        },
    );

    fs_setup.set_block_nonce(20);

    // random user tx to collect rewards

    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 12, 1);
    fs_setup.set_block_epoch(12);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            6,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(15);

    fs_setup.set_user_energy(&user_address, 10_000, 15, 10);

    let expected_rewards_amount = full_position_base_rewards / 2 * 2; // half remaining position * 2 times the 10 block period
    let half_position_boosted_rewards = boosted_rewards_user / 2;
    let remaining_expected_rewards = expected_rewards_amount + half_position_boosted_rewards;
    let final_expected_farming_token_balance_user =
        expected_farming_token_balance_user + rust_biguint!(remaining_expected_rewards);
    let expected_reward_per_share = 600_000;
    fs_setup.claim_rewards(
        &user_address,
        farm_in_amount / 2,
        2,
        remaining_expected_rewards,
        &final_expected_farming_token_balance_user,
        &final_expected_farming_token_balance_user,
        8,
        expected_reward_per_share,
    );
}

#[test]
fn farm_staking_claim_boosted_rewards_for_user_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&fs_setup.user_address.clone(), 10_000, 0, 10);

    let farm_in_amount = 100_000_000;
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 1, 0, 0);
    fs_setup.check_farm_token_supply(farm_in_amount);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &fs_setup.user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_nonce(10);

    // random user tx to collect rewards
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    let user_address = fs_setup.user_address.clone();
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 5, 1);
    fs_setup.set_block_epoch(5);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(8);

    fs_setup.set_user_energy(&user_address, 10_000, 8, 10);

    // value taken from the "test_unstake_farm" test
    // originally, it was 40, but since 25% of the rewards go to boosted yields
    // rewards are now only 3/4 * 40 = 30
    //
    // 10 reserved for boosted yields -> 30 + 10
    let expected_boosted_reward_token_out = 10;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_boosted_reward_token_out);

    // Random_user claim boosted rewards for user
    let rand_user_reward_balance = 4_999_999_990u64;
    fs_setup.b_mock.check_esdt_balance(
        &rand_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(rand_user_reward_balance),
    );
    fs_setup.allow_external_claim_rewards(&user_address, true);
    fs_setup.claim_boosted_rewards_for_user(
        &user_address,
        &rand_user,
        expected_boosted_reward_token_out,
        &expected_farming_token_balance,
    );
    fs_setup.b_mock.check_esdt_balance(
        &rand_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(rand_user_reward_balance),
    );

    fs_setup.check_farm_token_supply(farm_in_amount);

    // User removes the allowance of claim boosted rewards
    fs_setup.allow_external_claim_rewards(&user_address, false);
    fs_setup.claim_boosted_rewards_for_user_expect_error(&user_address, &rand_user);
}

#[test]
fn farm_staking_full_position_boosted_rewards_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&fs_setup.user_address.clone(), 10_000, 0, 10);

    let farm_in_amount = 50_000_000;
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 2, 0, 0);
    fs_setup.check_farm_token_supply(farm_in_amount * 2);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &fs_setup.user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_nonce(10);

    // random user tx to collect rewards

    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 5, 1);
    fs_setup.set_block_epoch(5);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            4,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(8);

    fs_setup.set_user_energy(&fs_setup.user_address.clone(), 10_000, 8, 10);

    let expected_base_rewards = 15;
    let expected_boosted_rewards = 10;
    let mut expected_farming_token_balance = rust_biguint!(
        USER_TOTAL_RIDE_TOKENS - (farm_in_amount * 2)
            + expected_base_rewards
            + expected_boosted_rewards
    );
    let expected_reward_per_share = 300_000; // from 400_000 -> 300_000

    // Should receive half base rewards and full boosted rewards
    fs_setup.claim_rewards(
        &user_address,
        farm_in_amount,
        2,
        expected_base_rewards + expected_boosted_rewards,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        6,
        expected_reward_per_share,
    );

    // Should receive half base rewards and no boosted rewards
    expected_farming_token_balance += expected_base_rewards;
    fs_setup.claim_rewards(
        &user_address,
        farm_in_amount,
        3,
        expected_base_rewards,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        7,
        expected_reward_per_share,
    );
    fs_setup.check_farm_token_supply(farm_in_amount * 2);
}

#[test]
fn position_owner_change_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let first_user = fs_setup.user_address.clone();
    let second_user = fs_setup.user_address2.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&first_user, 10_000, 0, 10);
    fs_setup.set_user_energy(&second_user, 5_000, 0, 10);

    let farm_in_amount = 10_000_000;
    let half_farm_in_amount = farm_in_amount / 2;
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 2, 0, 0);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 3, 0, 0);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 4, 0, 0);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 5, 0, 0);

    fs_setup.check_farm_token_supply(farm_in_amount * 5);

    fs_setup.check_user_total_farm_position(&first_user, farm_in_amount * 5);
    fs_setup.check_user_total_farm_position(&second_user, 0);

    // First user transfers 5 position to second user
    fs_setup.send_position(&first_user, &second_user, 1, farm_in_amount, 0);
    fs_setup.send_position(&first_user, &second_user, 2, farm_in_amount, 0);
    fs_setup.send_position(&first_user, &second_user, 3, farm_in_amount, 0);
    fs_setup.send_position(&first_user, &second_user, 4, farm_in_amount, 0);
    fs_setup.send_position(&first_user, &second_user, 5, farm_in_amount, 0);

    // Total farm position unchanged as users only transfered the farm positions
    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 10);
    fs_setup.check_user_total_farm_position(&second_user, 0);

    let additional_farm_tokens = [TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 1,
        value: rust_biguint!(half_farm_in_amount),
    }];

    fs_setup.stake_farm(
        &second_user,
        farm_in_amount,
        &additional_farm_tokens,
        6,
        0,
        0,
    );

    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 9);
    fs_setup.check_user_total_farm_position(&second_user, farm_in_amount + half_farm_in_amount);

    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    // random user tx to collect rewards

    fs_setup.set_user_energy(&rand_user, 1, 5, 1);
    fs_setup.set_block_epoch(5);
    fs_setup.set_block_nonce(10);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(8);

    fs_setup.set_user_energy(&first_user, 10_000, 8, 10);
    fs_setup.set_user_energy(&second_user, 5_000, 8, 10);

    // Second user claims with half position from first user
    let mut rewards = 2;
    let mut expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + rewards);
    fs_setup.claim_rewards(
        &second_user,
        half_farm_in_amount,
        2,
        rewards,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        9,
        250_000,
    );

    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 8);
    fs_setup.check_user_total_farm_position(&second_user, farm_in_amount + half_farm_in_amount * 2);

    // random user tx to collect rewards
    fs_setup.set_user_energy(&rand_user, 1, 12, 1);
    fs_setup.set_block_epoch(12);
    fs_setup.set_block_nonce(20);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            10,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(15);

    fs_setup.set_user_energy(&first_user, 10_000, 15, 10);
    fs_setup.set_user_energy(&second_user, 5_000, 15, 10);

    // Second user exits with half position from first user
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &second_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(half_farm_in_amount),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    rewards += 3;
    expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + rewards);
    fs_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &expected_farming_token_balance,
    );

    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 7);
    fs_setup.check_user_total_farm_position(&second_user, farm_in_amount + half_farm_in_amount * 2);

    // First user claim boosted rewards
    let first_user_expected_boosted_reward_token_out = 5;
    let first_user_expected_farming_token_balance = rust_biguint!(
        USER_TOTAL_RIDE_TOKENS - farm_in_amount * 5 + first_user_expected_boosted_reward_token_out
    );
    fs_setup.claim_boosted_rewards_for_user(
        &first_user,
        &first_user,
        first_user_expected_boosted_reward_token_out,
        &first_user_expected_farming_token_balance,
    );

    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 7);
    fs_setup.check_user_total_farm_position(&second_user, farm_in_amount + half_farm_in_amount * 2);

    // random user tx to collect rewards
    fs_setup.set_user_energy(&rand_user, 1, 12, 1);
    fs_setup.set_block_epoch(20);
    fs_setup.set_block_nonce(30);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            13,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(22);

    fs_setup.set_user_energy(&first_user, 10_000, 22, 10);
    fs_setup.set_user_energy(&second_user, 5_000, 22, 10);

    // Second user merges half own position with 2 x half position from first user
    // We send the payment from first user first,
    // to see that the original caller is correctly updated as second user
    let farm_tokens = [
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: 4,
            value: rust_biguint!(half_farm_in_amount),
        },
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: 6,
            value: rust_biguint!(half_farm_in_amount),
        },
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: 5,
            value: rust_biguint!(half_farm_in_amount),
        },
    ];
    fs_setup
        .b_mock
        .execute_esdt_multi_transfer(&second_user, &fs_setup.farm_wrapper, &farm_tokens, |sc| {
            let _ = sc.merge_farm_tokens_endpoint();
        })
        .assert_ok();

    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(0),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(half_farm_in_amount * 3),
        original_owner: managed_address!(&second_user), // Check that second user is original owner
    };
    fs_setup.b_mock.check_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        15,
        &rust_biguint!(half_farm_in_amount * 3),
        Some(&expected_attributes),
    );
    rewards += 1;
    expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + rewards);
    fs_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &expected_farming_token_balance,
    );

    fs_setup.check_user_total_farm_position(&first_user, half_farm_in_amount * 5);
    fs_setup.check_user_total_farm_position(&second_user, farm_in_amount + half_farm_in_amount * 4);
}

#[test]
fn farm_staking_farm_position_migration_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user = fs_setup.user_address.clone();

    let farm_in_amount = 10_000_000;
    let half_farm_in_amount = farm_in_amount / 2;
    fs_setup.stake_farm(&user, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&user, farm_in_amount, &[], 2, 0, 0);
    fs_setup.stake_farm(&user, farm_in_amount, &[], 3, 0, 0);
    fs_setup.stake_farm(&user, farm_in_amount, &[], 4, 0, 0);
    fs_setup.check_user_total_farm_position(&user, farm_in_amount * 4);

    // Simulate migration by resetting the user total farm position
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                sc.user_total_farm_position(&managed_address!(&user))
                    .set(managed_biguint!(0u64));

                sc.farm_position_migration_nonce().set(5);
            },
        )
        .assert_ok();

    fs_setup.check_user_total_farm_position(&user, 0);

    let mut expected_total_farm_position = 0u64;
    let additional_farm_tokens = [TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 1,
        value: rust_biguint!(half_farm_in_amount),
    }];

    // Check enter farm with half old position additional payment
    fs_setup.stake_farm(&user, farm_in_amount, &additional_farm_tokens, 5, 0, 0);
    expected_total_farm_position += farm_in_amount + half_farm_in_amount;
    fs_setup.check_user_total_farm_position(&user, expected_total_farm_position);

    // Check claim with half old position
    let expected_farming_token_balance = rust_biguint!(4_949_999_990u64);
    fs_setup.claim_rewards(
        &user,
        half_farm_in_amount,
        2,
        0,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        6,
        0,
    );
    expected_total_farm_position += half_farm_in_amount;
    fs_setup.check_user_total_farm_position(&user, expected_total_farm_position);

    // Check exit with half old position
    fs_setup.unstake_farm(
        &user,
        half_farm_in_amount,
        3,
        0,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        7,
        half_farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: MIN_UNBOND_EPOCHS,
        },
    );
    fs_setup.check_user_total_farm_position(&user, expected_total_farm_position);

    // Check compound with half old position
    fs_setup.compound_rewards(
        &user,
        4,
        half_farm_in_amount,
        &[],
        8,
        half_farm_in_amount,
        0,
        0,
    );
    expected_total_farm_position += half_farm_in_amount;
    fs_setup.check_user_total_farm_position(&user, expected_total_farm_position);
}

#[test]
fn boosted_rewards_config_change_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let first_user = fs_setup.user_address.clone();
    let second_user = fs_setup.user_address2.clone();
    let third_user = fs_setup
        .b_mock
        .create_user_account(&rust_biguint!(100_000_000));
    fs_setup.b_mock.set_esdt_balance(
        &third_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    let mut first_user_total_rewards = 0u64;
    let mut second_user_total_rewards = 0u64;
    let mut third_user_total_rewards = 0u64;

    let farm_in_amount = 10_000_000;
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&second_user, farm_in_amount, &[], 2, 0, 0);
    fs_setup.stake_farm(&third_user, farm_in_amount, &[], 3, 0, 0);

    fs_setup.set_user_energy(&first_user, 10_000, 0, 10);
    fs_setup.set_user_energy(&second_user, 10_000, 0, 10);
    fs_setup.set_user_energy(&third_user, 10_000, 0, 10);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &second_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &third_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 6, 1);
    fs_setup.set_block_epoch(6);
    fs_setup.set_block_nonce(100);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(7);
    fs_setup.set_user_energy(&first_user, 10_000, 7, 10);
    fs_setup.set_user_energy(&second_user, 10_000, 7, 10);
    fs_setup.set_user_energy(&third_user, 10_000, 7, 10);

    // First user claims
    let mut base_rewards1 = 33;
    let mut boosted_rewards1 = 0;
    let mut expected_reward_token_out = base_rewards1 + boosted_rewards1;
    first_user_total_rewards += expected_reward_token_out;
    let mut expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let mut expected_reward_per_share = 3_333_333u64;
    fs_setup.claim_rewards(
        &first_user,
        farm_in_amount,
        4,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        9,
        expected_reward_per_share,
    );

    // Boosted rewards config is added
    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    // random user tx to collect rewards
    fs_setup.set_user_energy(&rand_user, 1, 13, 1);
    fs_setup.set_block_epoch(13);
    fs_setup.set_block_nonce(200);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            10,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(14);
    fs_setup.set_user_energy(&first_user, 10_000, 14, 10);
    fs_setup.set_user_energy(&second_user, 10_000, 14, 10);
    fs_setup.set_user_energy(&third_user, 10_000, 14, 10);

    // First and second users claim
    base_rewards1 = 25;
    boosted_rewards1 = 8;
    expected_reward_token_out = base_rewards1 + boosted_rewards1;
    first_user_total_rewards += expected_reward_token_out;
    expected_farming_token_balance += expected_reward_token_out;
    expected_reward_per_share = 5_833_333u64;
    fs_setup.claim_rewards(
        &first_user,
        farm_in_amount,
        9,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        12,
        expected_reward_per_share,
    );

    let mut base_rewards2 = 33 + 25;
    let mut boosted_rewards2 = 8;
    let mut expected_reward_token_out2 = base_rewards2 + boosted_rewards2;
    second_user_total_rewards += expected_reward_token_out2;
    let mut expected_farming_token_balance2 =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out2);
    fs_setup.claim_rewards(
        &second_user,
        farm_in_amount,
        5,
        expected_reward_token_out2,
        &expected_farming_token_balance2,
        &expected_farming_token_balance2,
        13,
        expected_reward_per_share,
    );

    // Boosted rewards config is updated
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE * 2); // 50%

    // random user tx to collect rewards
    fs_setup.set_user_energy(&rand_user, 1, 20, 1);
    fs_setup.set_block_epoch(20);
    fs_setup.set_block_nonce(300);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            14,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(21);
    fs_setup.set_user_energy(&first_user, 10_000, 21, 10);
    fs_setup.set_user_energy(&second_user, 10_000, 21, 10);
    fs_setup.set_user_energy(&third_user, 10_000, 21, 10);

    // All users claim - boosted rewards 50%
    base_rewards1 = 16;
    boosted_rewards1 = 16;
    expected_reward_token_out = base_rewards1 + boosted_rewards1;
    first_user_total_rewards += expected_reward_token_out;
    expected_farming_token_balance += expected_reward_token_out;
    expected_reward_per_share = 7_499_999u64;
    fs_setup.claim_rewards(
        &first_user,
        farm_in_amount,
        12,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        16,
        expected_reward_per_share,
    );

    base_rewards2 = 16;
    boosted_rewards2 = 16;
    expected_reward_token_out2 = base_rewards2 + boosted_rewards2;
    second_user_total_rewards += expected_reward_token_out2;
    expected_farming_token_balance2 += expected_reward_token_out2;
    fs_setup.claim_rewards(
        &second_user,
        farm_in_amount,
        13,
        expected_reward_token_out2,
        &expected_farming_token_balance2,
        &expected_farming_token_balance2,
        17,
        expected_reward_per_share,
    );

    let base_rewards3 = 74;
    let boosted_rewards3 = 24;
    let expected_reward_token_out3 = base_rewards3 + boosted_rewards3;
    third_user_total_rewards += expected_reward_token_out3;
    let expected_farming_token_balance3 =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out3);
    fs_setup.claim_rewards(
        &third_user,
        farm_in_amount,
        6,
        expected_reward_token_out3,
        &expected_farming_token_balance3,
        &expected_farming_token_balance3,
        18,
        expected_reward_per_share,
    );

    assert!(
        first_user_total_rewards == second_user_total_rewards
            && first_user_total_rewards == third_user_total_rewards
    );
}

#[test]
fn claim_only_boosted_rewards_per_week_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let first_user = fs_setup.user_address.clone();
    let farm_in_amount = 100_000_000;

    fs_setup.set_user_energy(&first_user, 10_000, 0, 10);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 1, 0, 0);

    fs_setup.check_farm_token_supply(farm_in_amount);
    fs_setup.check_farm_rps(0u64);

    fs_setup.b_mock.set_block_nonce(100);
    fs_setup.b_mock.set_block_epoch(6);
    fs_setup.set_user_energy(&first_user, 1_000, 6, 1);

    // Reset user balance
    fs_setup
        .b_mock
        .set_esdt_balance(&first_user, FARMING_TOKEN_ID, &rust_biguint!(0));

    // random user tx to collect rewards
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 6, 1);
    fs_setup.stake_farm(&rand_user, 10, &[], 2, 3_000_000u64, 0);
    fs_setup.unstake_farm_no_checks(&rand_user, 10, 2);

    let farm_rps_increase = 3_000_000u64;
    let mut current_farm_rps = 0;
    current_farm_rps += farm_rps_increase;
    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 13, 1);
    fs_setup.b_mock.set_block_nonce(200);
    fs_setup.b_mock.set_block_epoch(13);

    let boosted_rewards_for_week = 100;
    fs_setup.claim_boosted_rewards_for_user(
        &first_user,
        &first_user,
        boosted_rewards_for_week,
        &rust_biguint!(boosted_rewards_for_week),
    );

    current_farm_rps += farm_rps_increase;
    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 15, 1);
    fs_setup.b_mock.set_block_nonce(300);
    fs_setup.b_mock.set_block_epoch(15);
    fs_setup.claim_boosted_rewards_for_user(
        &first_user,
        &first_user,
        boosted_rewards_for_week,
        &rust_biguint!(boosted_rewards_for_week * 2),
    );

    current_farm_rps += farm_rps_increase;
    fs_setup.check_farm_rps(current_farm_rps);
    fs_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(boosted_rewards_for_week * 2),
    );

    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(0),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(farm_in_amount),
        original_owner: managed_address!(&first_user),
    };

    fs_setup.b_mock.check_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(farm_in_amount),
        Some(&expected_attributes),
    );
}

#[test]
fn claim_rewards_per_week_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let first_user = fs_setup.user_address.clone();
    let farm_in_amount = 100_000_000;

    fs_setup.set_user_energy(&first_user, 10_000, 0, 10);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 1, 0, 0);

    fs_setup.check_farm_token_supply(farm_in_amount);
    fs_setup.check_farm_rps(0u64);

    fs_setup.b_mock.set_block_nonce(100);
    fs_setup.b_mock.set_block_epoch(6);
    fs_setup.set_user_energy(&first_user, 1_000, 6, 1);

    // Reset user balance
    fs_setup
        .b_mock
        .set_esdt_balance(&first_user, FARMING_TOKEN_ID, &rust_biguint!(0));

    // random user tx to collect rewards
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 6, 1);
    fs_setup.stake_farm(&rand_user, 10, &[], 2, 3_000_000u64, 0);
    fs_setup.unstake_farm_no_checks(&rand_user, 10, 2);

    let farm_rps_increase = 3_000_000u64;
    let mut current_farm_rps = 0;
    current_farm_rps += farm_rps_increase;
    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 13, 1);
    fs_setup.b_mock.set_block_nonce(200);
    fs_setup.b_mock.set_block_epoch(13);

    let base_rewards_for_week = 300;
    let boosted_rewards_for_week = 100;

    current_farm_rps += farm_rps_increase;
    let mut user_rewards_balance = base_rewards_for_week * 2 + boosted_rewards_for_week;
    fs_setup.claim_rewards(
        &first_user,
        farm_in_amount,
        1,
        base_rewards_for_week * 2 + boosted_rewards_for_week,
        &rust_biguint!(user_rewards_balance),
        &rust_biguint!(user_rewards_balance), // user balance has bet set to 0 at the start
        4,
        current_farm_rps,
    );

    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 15, 1);
    fs_setup.b_mock.set_block_nonce(300);
    fs_setup.b_mock.set_block_epoch(15);

    current_farm_rps += farm_rps_increase;
    user_rewards_balance += base_rewards_for_week + boosted_rewards_for_week;
    fs_setup.claim_rewards(
        &first_user,
        farm_in_amount,
        4,
        base_rewards_for_week + boosted_rewards_for_week,
        &rust_biguint!(user_rewards_balance),
        &rust_biguint!(user_rewards_balance),
        5,
        current_farm_rps,
    );

    fs_setup.check_farm_rps(current_farm_rps);

    fs_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(user_rewards_balance),
    );
}

#[test]
fn claim_boosted_rewards_with_zero_position_test() {
    DebugApi::dummy();
    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let first_user = fs_setup.user_address.clone();
    let farm_in_amount = 100_000_000;

    fs_setup.set_user_energy(&first_user, 10_000, 0, 10);
    fs_setup.stake_farm(&first_user, farm_in_amount, &[], 1, 0, 0);

    fs_setup.check_farm_token_supply(farm_in_amount);
    fs_setup.check_farm_rps(0u64);

    fs_setup.b_mock.set_block_nonce(100);
    fs_setup.b_mock.set_block_epoch(6);
    fs_setup.set_user_energy(&first_user, 1_000, 6, 1);

    // Reset user balance
    fs_setup
        .b_mock
        .set_esdt_balance(&first_user, FARMING_TOKEN_ID, &rust_biguint!(0));

    // tx to collect rewards
    let second_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &second_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&second_user, 1, 6, 1);
    fs_setup.stake_farm(&second_user, 10, &[], 2, 3_000_000u64, 0);
    fs_setup.unstake_farm_no_checks(&second_user, 10, 2);

    let farm_rps_increase = 3_000_000u64;
    let mut current_farm_rps = 0;
    current_farm_rps += farm_rps_increase;
    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 13, 1);
    fs_setup.b_mock.set_block_nonce(200);
    fs_setup.b_mock.set_block_epoch(13);

    let boosted_rewards_for_week = 100;

    fs_setup
        .b_mock
        .execute_tx(
            &second_user,
            &fs_setup.farm_wrapper,
            &rust_biguint!(0u64),
            |sc| {
                sc.claim_boosted_rewards(OptionalValue::Some(managed_address!(&second_user)));
            },
        )
        .assert_error(4, "User total farm position is empty!");

    fs_setup.check_farm_rps(current_farm_rps);

    // advance 1 week
    fs_setup.set_user_energy(&first_user, 1_000, 15, 1);
    fs_setup.b_mock.set_block_nonce(300);
    fs_setup.b_mock.set_block_epoch(15);
    fs_setup.claim_boosted_rewards_for_user(
        &first_user,
        &first_user,
        boosted_rewards_for_week,
        &rust_biguint!(boosted_rewards_for_week),
    );

    current_farm_rps += farm_rps_increase * 2;
    fs_setup.check_farm_rps(current_farm_rps);
    fs_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(boosted_rewards_for_week),
    );

    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(0),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(farm_in_amount),
        original_owner: managed_address!(&first_user),
    };

    fs_setup.b_mock.check_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(farm_in_amount),
        Some(&expected_attributes),
    );
}

#[test]
fn test_multiple_positions_on_behalf() {
    DebugApi::dummy();

    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    fs_setup.set_boosted_yields_factors();
    let mut block_nonce = 0u64;
    fs_setup.b_mock.set_block_nonce(block_nonce);

    // new external user
    let external_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.set_user_energy(&external_user, 1_000, 1, 1);

    // authorized address
    let farm_token_amount = 100_000_000;
    let authorized_address = fs_setup.user_address.clone();
    fs_setup.b_mock.set_esdt_balance(
        &authorized_address,
        FARMING_TOKEN_ID,
        &rust_biguint!(farm_token_amount * 2),
    );

    fs_setup.whitelist_address_on_behalf(&external_user, &authorized_address);

    fs_setup.check_farm_token_supply(0);
    fs_setup.stake_farm_on_behalf(&authorized_address, &external_user, farm_token_amount, 0, 0);
    fs_setup.check_farm_token_supply(farm_token_amount);

    let block_nonce_diff = 10u64;
    block_nonce += block_nonce_diff;
    fs_setup.b_mock.set_block_nonce(block_nonce);

    let base_rewards = 30u64;
    let boosted_rewards = 10u64;
    let total_rewards = base_rewards + boosted_rewards;

    // Only base rewards are given
    fs_setup
        .b_mock
        .check_esdt_balance(&external_user, REWARD_TOKEN_ID, &rust_biguint!(0));
    fs_setup.claim_rewards_on_behalf(&authorized_address, 1, farm_token_amount);
    fs_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards),
    );

    // random tx on end of week 1, to cummulate rewards
    fs_setup.b_mock.set_block_epoch(6);
    let temp_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &temp_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );
    fs_setup.set_user_energy(&external_user, 1_000, 6, 1);
    fs_setup.set_user_energy(&temp_user, 1, 6, 1);
    fs_setup.stake_farm(&temp_user, 10, &[], 3, 300_000u64, 0);
    fs_setup.unstake_farm_no_checks(&temp_user, 10, 3);

    // advance 1 week
    block_nonce += block_nonce_diff;
    fs_setup.b_mock.set_block_nonce(block_nonce);
    fs_setup.b_mock.set_block_epoch(10);
    fs_setup.set_user_energy(&external_user, 1_000, 10, 1);

    // enter farm again for the same user (with additional payment)
    fs_setup.check_farm_token_supply(farm_token_amount);
    fs_setup.stake_farm_on_behalf(
        &authorized_address,
        &external_user,
        farm_token_amount,
        2, // nonce 2 as the user already claimed with this position
        farm_token_amount,
    );
    fs_setup.check_farm_token_supply(farm_token_amount * 2);
    fs_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards + boosted_rewards),
    );

    fs_setup.claim_rewards_on_behalf(&authorized_address, 5, farm_token_amount * 2);
    fs_setup.check_farm_token_supply(farm_token_amount * 2);
    fs_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(total_rewards + base_rewards),
    );

    let farm_token_attributes: StakingFarmTokenAttributes<DebugApi> = StakingFarmTokenAttributes {
        reward_per_share: managed_biguint!(600_000u64),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(farm_token_amount * 2),
        original_owner: managed_address!(&external_user),
    };

    fs_setup.b_mock.check_nft_balance(
        &authorized_address,
        FARM_TOKEN_ID,
        6,
        &rust_biguint!(farm_token_amount * 2),
        Some(&farm_token_attributes),
    );
}

#[test]
fn owner_claim_undist_rewards_test() {
    DebugApi::dummy();

    let mut fs_setup = FarmStakingSetup::new(
        farm_staking::contract_obj,
        energy_factory::contract_obj,
        permissions_hub::contract_obj,
    );

    let user_address = fs_setup.user_address.clone();
    let user_address2 = fs_setup.user_address2.clone();

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&user_address, 9_800, 0, 100);
    fs_setup.set_user_energy(&user_address2, 4_900, 0, 350);

    let farm_in_amount = 100_000_000;
    fs_setup.stake_farm(&user_address, farm_in_amount, &[], 1, 0, 0);
    fs_setup.stake_farm(&user_address2, farm_in_amount, &[], 2, 0, 0);
    fs_setup.check_farm_token_supply(farm_in_amount * 2);

    // claim to get energy registered
    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address2,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            2,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // random user tx to collect rewards - week 1
    let rand_user = fs_setup.b_mock.create_user_account(&rust_biguint!(0));
    fs_setup.b_mock.set_esdt_balance(
        &rand_user,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    fs_setup.set_user_energy(&rand_user, 1, 6, 1);
    fs_setup.set_block_epoch(6);
    fs_setup.set_block_nonce(10);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.stake_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &rand_user,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            5,
            &rust_biguint!(10),
            |sc| {
                let _ = sc.unstake_farm(OptionalValue::None);
            },
        )
        .assert_ok();

    // first user claim - week 2
    fs_setup.set_block_epoch(13);
    fs_setup.set_block_nonce(20);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // first user claim - week 3
    fs_setup.set_block_epoch(20);
    fs_setup.set_block_nonce(30);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // first user claim - week 4
    fs_setup.set_block_epoch(27);
    fs_setup.set_block_nonce(40);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            8,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // try to collect rewards too early - should fail
    fs_setup
        .b_mock
        .execute_tx(
            &fs_setup.owner_address,
            &fs_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.collect_undistributed_boosted_rewards();
            },
        )
        .assert_error(4, "Current week must be higher than the week offset");

    // first user claim - week 5
    fs_setup.set_block_epoch(34);
    fs_setup.set_block_nonce(50);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            9,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // first user claim - week 6
    fs_setup.set_block_epoch(41);
    fs_setup.set_block_nonce(50);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            10,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    // first user claim - week 7
    fs_setup.set_block_epoch(48);
    fs_setup.set_block_nonce(60);

    fs_setup
        .b_mock
        .execute_esdt_transfer(
            &user_address,
            &fs_setup.farm_wrapper,
            FARM_TOKEN_ID,
            11,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(49);

    // Verify remaining rewards state
    fs_setup
        .b_mock
        .execute_query(&fs_setup.farm_wrapper, |sc| {
            // Check remaining rewards for weeks 1-3
            let remaining1 = sc.remaining_boosted_rewards_to_distribute(1).get();
            let remaining2 = sc.remaining_boosted_rewards_to_distribute(2).get();
            let remaining3 = sc.remaining_boosted_rewards_to_distribute(3).get();

            // We should have some undistributed rewards
            assert!(
                remaining1 > 0 && remaining2 > 0 && remaining3 > 0,
                "Should have remaining rewards to distribute in weeks 1 to 3"
            );

            // Check last_collect_undist_week is not set yet
            let last_collect = sc.last_collect_undist_week().get();
            assert_eq!(last_collect, 0, "Last collect week should be 0 initially");
        })
        .assert_ok();

    // owner collect undist rewards
    let owner = fs_setup.owner_address.clone();
    fs_setup
        .b_mock
        .execute_tx(
            &fs_setup.owner_address,
            &fs_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                let undist_rewards = sc.collect_undistributed_boosted_rewards();
                assert_eq!(undist_rewards, 22);

                // Verify last_collect_undist_week was updated
                let last_collect = sc.last_collect_undist_week().get();
                assert!(last_collect == 4, "Last collect week should be updated");

                // Verify remaining rewards for week 1 are now zero
                let remaining1 = sc.remaining_boosted_rewards_to_distribute(1).get();
                assert_eq!(remaining1, 0u64, "Week 1 rewards should now be zero");

                // Verify remaining rewards for week 2 are now zero
                let remaining2 = sc.remaining_boosted_rewards_to_distribute(2).get();
                assert_eq!(remaining2, 0u64, "Week 2 rewards should now be zero");

                // Verify remaining rewards for week 3 are now zero
                let remaining3 = sc.remaining_boosted_rewards_to_distribute(3).get();
                assert_eq!(remaining3, 0u64, "Week 3 rewards should now be zero");
            },
        )
        .assert_ok();

    // check owner received tokens
    fs_setup
        .b_mock
        .check_esdt_balance(&owner, REWARD_TOKEN_ID, &rust_biguint!(22));
}
