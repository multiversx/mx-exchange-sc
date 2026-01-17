#![allow(deprecated)]

mod farm_setup;

use config::ConfigModule;
use farm::Farm;
use farm_setup::multi_user_farm_setup::{MultiUserFarmSetup, BOOSTED_YIELDS_PERCENTAGE};
use multiversx_sc_scenario::{managed_biguint, rust_biguint, DebugApi};
use rewards::RewardsModule;

use crate::farm_setup::multi_user_farm_setup::DIV_SAFETY;

#[test]
fn test_migration_reward_calculation_precision() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    // Set up boosted yields configuration
    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // Setup users and amounts
    let first_user = farm_setup.first_user.clone();
    let second_user = farm_setup.second_user.clone();
    let first_user_amount = 75_000_000u64; // 75% of total
    let second_user_amount = 25_000_000u64; // 25% of total

    // Initial setup
    let initial_block = 1000u64;
    let initial_timestamp = 6000u64; // 1000 * 6
    farm_setup.b_mock.set_block_nonce(initial_block);
    farm_setup.b_mock.set_block_timestamp(initial_timestamp);

    // Users enter farm
    farm_setup.enter_farm(&first_user, first_user_amount);
    farm_setup.enter_farm(&second_user, second_user_amount);

    // Setup block-based rewards
    let per_block_reward = 1_200u64;
    farm_setup.simulate_per_block_migration_storage(per_block_reward, initial_block);

    // Advance exactly 100 blocks (600 seconds)
    let blocks_to_advance = 100u64;
    let new_block = initial_block + blocks_to_advance;
    let new_timestamp = initial_timestamp + (blocks_to_advance * 6);
    let new_epoch = 10; // new week
    farm_setup.b_mock.set_block_nonce(new_block);
    farm_setup.b_mock.set_block_timestamp(new_timestamp);
    farm_setup.b_mock.set_block_epoch(new_epoch);

    // Calculate expected rewards before upgrade
    let total_rewards_generated = per_block_reward * blocks_to_advance; // 120_000
    let base_farm_percentage = 10_000 - BOOSTED_YIELDS_PERCENTAGE; // 7_500 (75%)
    let base_farm_rewards = total_rewards_generated * base_farm_percentage / 10_000; // 90_000

    // Get initial state
    let rps_before = farm_setup.get_reward_per_share();

    // Execute upgrade
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.upgrade();
            },
        )
        .assert_ok();

    // Verify conversion
    farm_setup
        .b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            let per_second = sc.per_second_reward_amount().get();
            assert_eq!(
                per_second,
                managed_biguint!(per_block_reward / 6), // 200 per second
                "Per second reward should be exactly per_block / 6"
            );

            // Check rewards were properly aggregated
            let rps_after = sc.reward_per_share().get();
            let rps_increase = &rps_after - &managed_biguint!(rps_before);

            // RPS increase should be: base_farm_rewards * DIV_SAFETY / total_farm_supply
            let total_farm_supply = first_user_amount + second_user_amount;
            let expected_rps_increase = managed_biguint!(base_farm_rewards)
                * managed_biguint!(DIV_SAFETY)
                / managed_biguint!(total_farm_supply);

            assert_eq!(
                rps_increase, expected_rps_increase,
                "RPS increase should match expected calculation"
            );
        })
        .assert_ok();

    // Both users claim to verify they get correct proportions
    let first_user_rewards = farm_setup.claim_rewards(&first_user, 1, first_user_amount);
    let second_user_rewards = farm_setup.claim_rewards(&second_user, 2, second_user_amount);

    // Verify reward proportions
    let first_user_expected =
        base_farm_rewards * first_user_amount / (first_user_amount + second_user_amount);
    let second_user_expected =
        base_farm_rewards * second_user_amount / (first_user_amount + second_user_amount);

    assert_eq!(first_user_rewards, first_user_expected);

    assert_eq!(second_user_rewards, second_user_expected);

    // Verify total rewards claimed equals what was generated (minus boosted yields)
    let total_claimed = first_user_rewards + second_user_rewards;
    assert_eq!(total_claimed, base_farm_rewards);
}

#[test]
fn test_migration_with_no_pending_rewards() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );
    // Set up boosted yields configuration
    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let first_user = farm_setup.first_user.clone();
    let farm_in_amount = 100_000_000;

    // Setup at same block/timestamp
    let block = 1000u64;
    let timestamp = 6000u64;
    farm_setup.b_mock.set_block_nonce(block);
    farm_setup.b_mock.set_block_timestamp(timestamp);

    farm_setup.enter_farm(&first_user, farm_in_amount);

    // Setup block-based rewards but with last_reward_block = current block (no rewards to claim)
    let per_block_reward = 1_000u64;
    farm_setup.simulate_per_block_migration_storage(per_block_reward, block);

    let rps_before = farm_setup.get_reward_per_share();

    // Execute upgrade without advancing time
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.upgrade();
            },
        )
        .assert_ok();

    // Verify no rewards were generated during upgrade
    let rps_after = farm_setup.get_reward_per_share();
    assert_eq!(
        rps_before, rps_after,
        "RPS should not change when no rewards are pending"
    );

    // Verify storage was still properly migrated
    farm_setup
        .b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            let per_second = sc.per_second_reward_amount().get();
            assert_eq!(
                per_second,
                managed_biguint!(per_block_reward / 6),
                "Per second reward should still be set"
            );

            let last_timestamp = sc.last_reward_timestamp().get();
            assert_eq!(
                last_timestamp, timestamp,
                "Last timestamp should be set to current"
            );
        })
        .assert_ok();

    // Verify old storage cleared
    farm_setup.verify_old_storage_cleared();
}

#[test]
fn test_migration_continuity() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    // Set up boosted yields configuration
    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let user = farm_setup.first_user.clone();
    let rand_user = farm_setup.second_user.clone();
    let farm_amount = 100_000_000;

    // Initial setup
    let initial_block = 1000u64;
    let initial_timestamp = 6000u64;
    farm_setup.b_mock.set_block_nonce(initial_block);
    farm_setup.b_mock.set_block_timestamp(initial_timestamp);

    farm_setup.enter_farm(&user, farm_amount);

    // Setup block rewards
    let per_block_reward = 600u64; // 100 per second after conversion
    farm_setup.simulate_per_block_migration_storage(per_block_reward, initial_block);

    // Advance time before upgrade
    let pre_upgrade_blocks = 50u64;
    let pre_upgrade_timestamp = initial_timestamp + (pre_upgrade_blocks * 6);
    let pre_upgrade_epoch = 10; // New week
    farm_setup
        .b_mock
        .set_block_nonce(initial_block + pre_upgrade_blocks);
    farm_setup.b_mock.set_block_timestamp(pre_upgrade_timestamp);
    farm_setup.b_mock.set_block_epoch(pre_upgrade_epoch);

    // Record state
    let rps_pre_upgrade = farm_setup.get_reward_per_share();

    // Execute upgrade
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.upgrade();
            },
        )
        .assert_ok();

    let rps_post_upgrade = farm_setup.get_reward_per_share();

    // Advance time after upgrade (using timestamps now)
    let post_upgrade_seconds = 300u64;
    farm_setup.advance_time(post_upgrade_seconds);

    // Trigger reward calculation by entering farm with another user
    farm_setup.enter_farm(&rand_user, 1);

    let rps_final = farm_setup.get_reward_per_share();

    // Calculate expected RPS increases
    let pre_upgrade_rewards = per_block_reward * pre_upgrade_blocks; // Block-based
    let post_upgrade_rewards = (per_block_reward / 6) * post_upgrade_seconds; // Timestamp-based

    let base_percentage = 10_000 - BOOSTED_YIELDS_PERCENTAGE;
    let pre_upgrade_base = pre_upgrade_rewards * base_percentage / 10_000;
    let post_upgrade_base = post_upgrade_rewards * base_percentage / 10_000;

    let expected_rps_increase_pre = pre_upgrade_base * DIV_SAFETY / farm_amount;
    let expected_rps_increase_post = post_upgrade_base * DIV_SAFETY / farm_amount;

    // Verify continuity
    let actual_pre_upgrade_increase = rps_post_upgrade - rps_pre_upgrade;
    let actual_post_upgrade_increase = rps_final - rps_post_upgrade;

    assert_eq!(
        actual_pre_upgrade_increase, expected_rps_increase_pre,
        "Pre-upgrade rewards calculation mismatch"
    );

    assert_eq!(
        actual_post_upgrade_increase, expected_rps_increase_post,
        "Post-upgrade rewards calculation mismatch"
    );
}
