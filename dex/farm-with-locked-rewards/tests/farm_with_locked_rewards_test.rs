use common_structs::FarmTokenAttributes;
use farm_with_locked_rewards::Farm;
use multiversx_sc::{codec::Empty, imports::OptionalValue};
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint, DebugApi};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::farm_with_locked_rewards_setup::{
    FarmSetup, BOOSTED_YIELDS_PERCENTAGE, FARM_TOKEN_ID, LOCKED_REWARD_TOKEN_ID,
};

mod farm_with_locked_rewards_setup;

#[test]
fn farm_with_no_boost_no_proxy_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_user = farm_setup.second_user.clone();
    farm_setup.enter_farm(&second_user, second_farm_token_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    farm_setup.b_mock.set_block_nonce(10);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // calculate rewards - first user
    let first_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        entering_epoch: 0,
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(first_farm_token_amount),
        original_owner: managed_address!(&first_user),
    };
    let first_rewards_amt =
        farm_setup.calculate_rewards(&first_user, first_farm_token_amount, first_attributes);
    let first_expected_rewards_amt = first_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(first_rewards_amt, first_expected_rewards_amt);

    // calculate rewards - second user
    let second_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        entering_epoch: 0,
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(second_farm_token_amount),
        original_owner: managed_address!(&second_user),
    };
    let second_rewards_amt =
        farm_setup.calculate_rewards(&second_user, second_farm_token_amount, second_attributes);
    let second_expected_rewards_amt = second_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(second_rewards_amt, second_expected_rewards_amt);

    // first user claim
    let first_received_reward_amt =
        farm_setup.claim_rewards(&first_user, 1, first_farm_token_amount);
    assert_eq!(first_received_reward_amt, first_expected_rewards_amt);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(first_farm_token_amount),
            None,
        );

    farm_setup
        .b_mock
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &first_user,
            LOCKED_REWARD_TOKEN_ID,
            1,
            &rust_biguint!(first_received_reward_amt),
            None,
        );

    // second user claim
    let second_received_reward_amt =
        farm_setup.claim_rewards(&second_user, 2, second_farm_token_amount);
    assert_eq!(second_received_reward_amt, second_expected_rewards_amt);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            4,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup
        .b_mock
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &second_user,
            LOCKED_REWARD_TOKEN_ID,
            1, //nonce caching
            &rust_biguint!(second_received_reward_amt),
            None,
        );
}

#[test]
fn farm_with_boosted_yields_no_proxy_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);
    let third_user = farm_setup.third_user.clone();

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_user = farm_setup.second_user.clone();
    farm_setup.set_user_energy(&second_user, 4_000, 2, 1);
    farm_setup.enter_farm(&second_user, second_farm_token_amount);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 1, first_farm_token_amount);
    let _ = farm_setup.claim_rewards(&second_user, 2, second_farm_token_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(10);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 6, 1);
    farm_setup.set_user_energy(&third_user, 1, 6, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 5, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 10, 1);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // first user claim
    let first_base_farm_amt = first_farm_token_amount * 7_500 / total_farm_tokens;

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 1_000 / 5_000 + 2500 * 2 * 100_000_000 / 150_000_000) / (3 + 2)
    // (1500 + 3333) / (5) = 966
    let first_boosted_amt = 966; // 1000 energy & 100_000_000 farm tokens
    let first_total = first_base_farm_amt + first_boosted_amt;

    let first_receveived_reward_amt =
        farm_setup.claim_rewards(&first_user, 3, first_farm_token_amount);
    assert_eq!(first_receveived_reward_amt, first_total);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            6,
            &rust_biguint!(first_farm_token_amount),
            None,
        );

    farm_setup
        .b_mock
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &first_user,
            LOCKED_REWARD_TOKEN_ID,
            1,
            &rust_biguint!(first_receveived_reward_amt),
            None,
        );

    // second user claim
    let second_base_farm_amt = second_farm_token_amount * 7_500 / total_farm_tokens;

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 4000 / 5_000 + 2500 * 2 * 50_000_000 / 150_000_000) / (3 + 2)
    // (6000 + 1666) / (5) = 1533
    let second_boosted_amt = 1533; // 4000 energy & 50_000_000 farm tokens
    let second_total = second_base_farm_amt + second_boosted_amt;

    let second_receveived_reward_amt =
        farm_setup.claim_rewards(&second_user, 4, second_farm_token_amount);
    assert_eq!(second_receveived_reward_amt, second_total);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup
        .b_mock
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &second_user,
            LOCKED_REWARD_TOKEN_ID,
            1, //nonce caching
            &rust_biguint!(second_receveived_reward_amt),
            None,
        );
}

#[test]
fn total_farm_position_claim_with_locked_rewards_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let temp_user = farm_setup.third_user.clone();

    // first user enter farm
    let farm_in_amount = 50_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    farm_setup.b_mock.check_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(farm_in_amount),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(0),
            compounded_reward: managed_biguint!(0),
            entering_epoch: 2,
            current_farm_amount: managed_biguint!(farm_in_amount),
            original_owner: managed_address!(&first_user),
        }),
    );

    farm_setup.b_mock.check_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        2,
        &rust_biguint!(farm_in_amount),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(0),
            compounded_reward: managed_biguint!(0),
            entering_epoch: 2,
            current_farm_amount: managed_biguint!(farm_in_amount),
            original_owner: managed_address!(&first_user),
        }),
    );

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 1, farm_in_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(10);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 4, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);

    let total_farm_tokens = farm_in_amount * 2;

    // first user claim with half total position
    let first_base_farm_amt = farm_in_amount * 7_500 / total_farm_tokens;

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2_500 * 3 * 1_000 / 1_000 + 2_500 * 2 * 100_000_000 / 100_000_000) / (3 + 2)
    // (7_500 + 2_500) / (5) = 2_500
    let first_boosted_amt = 2_500; // 1000 energy & 100_000_000 farm tokens
    let first_total_rewards = first_base_farm_amt + first_boosted_amt;

    let first_received_reward_amt = farm_setup.claim_rewards(&first_user, 3, farm_in_amount);

    // Should be equal to half base generated rewards + full boosted generated rewards
    assert_eq!(first_received_reward_amt, first_total_rewards);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            5,
            &rust_biguint!(farm_in_amount),
            None,
        );

    // Check user receive locked rewards
    farm_setup
        .b_mock
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &first_user,
            LOCKED_REWARD_TOKEN_ID,
            1,
            &rust_biguint!(first_received_reward_amt),
            None,
        );
}

#[test]
fn claim_only_boosted_rewards_per_week_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let temp_user = farm_setup.third_user.clone();

    // first user enter farm
    let farm_in_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    farm_setup.check_farm_token_supply(farm_in_amount);
    farm_setup.check_farm_rps(0u64);

    farm_setup.b_mock.set_block_nonce(10);
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 2, 1);

    farm_setup.check_farm_rps(75_000_000u64);

    // advance 1 week
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.b_mock.set_block_nonce(20);
    farm_setup.b_mock.set_block_epoch(13);

    let boosted_rewards = 2_500;
    let second_week_received_reward_amt =
        farm_setup.claim_boosted_rewards_for_user(&first_user, &first_user, 1);

    assert_eq!(second_week_received_reward_amt, boosted_rewards);
    farm_setup.check_farm_rps(150_000_000u64);

    // advance 1 week
    farm_setup.set_user_energy(&first_user, 1_000, 15, 1);
    farm_setup.b_mock.set_block_nonce(30);
    farm_setup.b_mock.set_block_epoch(15);
    let third_week_received_reward_amt =
        farm_setup.claim_boosted_rewards_for_user(&first_user, &first_user, 1);

    assert_eq!(third_week_received_reward_amt, boosted_rewards);
    farm_setup.check_farm_rps(225_000_000u64);

    farm_setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_REWARD_TOKEN_ID,
        1,
        &rust_biguint!(boosted_rewards * 2),
        None,
    );
}

#[test]
fn claim_rewards_per_week_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let temp_user = farm_setup.third_user.clone();

    // first user enter farm
    let farm_in_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    farm_setup.check_farm_token_supply(farm_in_amount);
    farm_setup.check_farm_rps(0u64);

    farm_setup.b_mock.set_block_nonce(10);
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 2, 1);

    farm_setup.check_farm_rps(75_000_000u64);
    let base_rewards_per_week = 7_500;
    let boosted_rewards_per_week = 2_500;
    let total_rewards_per_week = base_rewards_per_week + boosted_rewards_per_week;

    // advance 1 week
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.b_mock.set_block_nonce(20);
    farm_setup.b_mock.set_block_epoch(13);

    let second_week_received_reward_amt = farm_setup.claim_rewards(&first_user, 1, farm_in_amount);

    assert_eq!(
        second_week_received_reward_amt,
        total_rewards_per_week + base_rewards_per_week
    );
    farm_setup.check_farm_rps(150_000_000u64);

    // advance 1 week
    farm_setup.set_user_energy(&first_user, 1_000, 15, 1);
    farm_setup.b_mock.set_block_nonce(30);
    farm_setup.b_mock.set_block_epoch(15);
    let third_week_received_reward_amt = farm_setup.claim_rewards(&first_user, 3, farm_in_amount);

    assert_eq!(third_week_received_reward_amt, total_rewards_per_week);
    farm_setup.check_farm_rps(225_000_000u64);

    farm_setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_REWARD_TOKEN_ID,
        1,
        &rust_biguint!(total_rewards_per_week * 2 + base_rewards_per_week),
        None,
    );
}

#[test]
fn claim_boosted_rewards_with_zero_position_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let temp_user = farm_setup.third_user.clone();

    // first user enter farm
    let farm_in_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    farm_setup.check_farm_token_supply(farm_in_amount);
    farm_setup.check_farm_rps(0u64);

    farm_setup.b_mock.set_block_nonce(10);
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 2, 1);

    farm_setup.check_farm_rps(75_000_000u64);

    // advance 1 week
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.b_mock.set_block_nonce(20);
    farm_setup.b_mock.set_block_epoch(13);

    farm_setup
        .b_mock
        .execute_tx(
            &temp_user,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.claim_boosted_rewards(OptionalValue::Some(managed_address!(&temp_user)));
            },
        )
        .assert_error(4, "User total farm position is empty!");

    farm_setup.check_farm_rps(75_000_000u64);

    // advance 1 week
    let boosted_rewards = 2_500;
    farm_setup.set_user_energy(&first_user, 1_000, 15, 1);
    farm_setup.b_mock.set_block_nonce(30);
    farm_setup.b_mock.set_block_epoch(15);
    let third_week_received_reward_amt =
        farm_setup.claim_boosted_rewards_for_user(&first_user, &first_user, 1);

    assert_eq!(third_week_received_reward_amt, boosted_rewards);
    farm_setup.check_farm_rps(225_000_000u64);

    farm_setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_REWARD_TOKEN_ID,
        1,
        &rust_biguint!(boosted_rewards),
        None,
    );
}

#[test]
fn claim_boosted_rewards_user_energy_not_registered_test() {
    DebugApi::dummy();

    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        timestamp_oracle::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let first_user = farm_setup.first_user.clone();
    let farm_in_amount = 100_000_000;

    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);

    // Attempt to claim boosted rewards without entering the farm
    farm_setup
        .b_mock
        .execute_tx(
            &first_user,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.claim_boosted_rewards(OptionalValue::Some(managed_address!(&first_user)));
            },
        )
        .assert_error(4, "User total farm position is empty!");

    // User enters the farm
    farm_setup.enter_farm(&first_user, farm_in_amount);

    // Now the user should be able to claim boosted rewards
    // Rewards computation is out of scope
    farm_setup.claim_boosted_rewards_for_user(&first_user, &first_user, 0);
}
