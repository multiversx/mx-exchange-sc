#![allow(deprecated)]

use common_structs::FarmTokenAttributes;
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint, DebugApi};

pub mod farm_setup;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_boosted_yields::boosted_yields_factors::{BoostedYieldsConfig, BoostedYieldsFactors};
use farm_setup::multi_user_farm_setup::*;
use permissions_module::{Permissions, PermissionsModule};
use week_timekeeping::WeekTimekeepingModule;
use weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo;

#[test]
fn farm_with_no_boost_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_farm_token_nonce = 1u64;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_farm_token_nonce = 2u64;
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
        farm_setup.claim_rewards(&first_user, first_farm_token_nonce, first_farm_token_amount);
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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_reward_amt),
    );

    // second user claim
    let second_received_reward_amt = farm_setup.claim_rewards(
        &second_user,
        second_farm_token_nonce,
        second_farm_token_amount,
    );
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

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_received_reward_amt),
    );
}

#[test]
fn farm_with_boosted_yields_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    let third_user = farm_setup.third_user.clone();
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
    farm_setup.exit_farm(&third_user, 5, 1, 1);

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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt),
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

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_receveived_reward_amt),
    );
}

#[test]
fn farm_change_boosted_yields_factors_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(10);

    let farm_addr = farm_setup.farm_wrapper.address_ref().clone();
    farm_setup
        .b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            let current_week = sc.get_current_week();
            let default_factors = BoostedYieldsFactors::<DebugApi> {
                max_rewards_factor: managed_biguint!(MAX_REWARDS_FACTOR),
                min_energy_amount: managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                min_farm_amount: managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                user_rewards_energy_const: managed_biguint!(USER_REWARDS_ENERGY_CONST),
                user_rewards_farm_const: managed_biguint!(USER_REWARDS_FARM_CONST),
            };

            let mut expected_config =
                BoostedYieldsConfig::new(current_week - 1, default_factors.clone());
            assert_eq!(expected_config, sc.boosted_yields_config().get());

            sc.add_permissions(managed_address!(&farm_addr), Permissions::all());
            sc.set_boosted_yields_factors(
                managed_biguint!(1u64),
                managed_biguint!(1u64),
                managed_biguint!(1u64),
                managed_biguint!(1u64),
                managed_biguint!(1u64),
            );

            let new_factors = BoostedYieldsFactors::<DebugApi> {
                max_rewards_factor: managed_biguint!(1u64),
                min_energy_amount: managed_biguint!(1u64),
                min_farm_amount: managed_biguint!(1u64),
                user_rewards_energy_const: managed_biguint!(1u64),
                user_rewards_farm_const: managed_biguint!(1u64),
            };
            expected_config.update(current_week, Some(new_factors.clone()));
            assert_eq!(expected_config, sc.boosted_yields_config().get());

            let factors_prev_week = expected_config
                .get_factors_for_week(current_week - 1)
                .clone();
            assert_eq!(factors_prev_week, default_factors);

            expected_config.update(current_week + 1, Some(new_factors));
            let factors_older_week = expected_config
                .get_factors_for_week(current_week - 2)
                .clone();
            assert_eq!(factors_prev_week, factors_older_week);
        })
        .assert_ok();
}

#[test]
fn farm_boosted_yields_claim_with_different_user_pos_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    let third_user = farm_setup.third_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    farm_setup.b_mock.check_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(first_farm_token_amount),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(0),
            compounded_reward: managed_biguint!(0),
            entering_epoch: 2,
            current_farm_amount: managed_biguint!(first_farm_token_amount),
            original_owner: managed_address!(&first_user),
        }),
    );

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
    farm_setup.exit_farm(&third_user, 5, 1, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 10, 1);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // transfer first user's position to second user
    farm_setup.b_mock.set_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        3,
        &rust_biguint!(first_farm_token_amount),
        &FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(0),
            compounded_reward: managed_biguint!(0),
            entering_epoch: 2,
            current_farm_amount: managed_biguint!(first_farm_token_amount),
            original_owner: managed_address!(&first_user),
        },
    );

    // second user claim with first user's pos
    // user will only receive rewards for base farm, no boosted rewards
    let second_base_farm_amt = first_farm_token_amount * 7_500 / total_farm_tokens;
    let second_receveived_reward_amt =
        farm_setup.claim_rewards(&second_user, 3, first_farm_token_amount);
    assert_eq!(second_receveived_reward_amt, second_base_farm_amt);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            6,
            &rust_biguint!(first_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_receveived_reward_amt),
    );
}

#[test]
fn farm_known_proxy_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_user = farm_setup.second_user.clone();
    farm_setup.enter_farm(&first_user, second_farm_token_amount);

    farm_setup.add_known_proxy(&first_user);

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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_reward_amt),
    );

    // first user claims for second user
    let second_received_reward_amt = farm_setup.claim_rewards_known_proxy(
        &second_user,
        2,
        second_farm_token_amount,
        &first_user,
    );
    assert_eq!(second_received_reward_amt, second_expected_rewards_amt);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            4,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_received_reward_amt + first_received_reward_amt),
    );
}

#[test]
fn farm_multiple_claim_weeks_with_collect_undistributed_rewards_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
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
    farm_setup.exit_farm(&third_user, 5, 1, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 10, 1);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // first user claim1
    let first_base_farm_amt = first_farm_token_amount * 7_500 / total_farm_tokens;

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 1_000 / 5_000 + 2500 * 2 * 100_000_000 / 150_000_000) / (3 + 2)
    // (1500 + 3333) / (5) = 966
    let first_boosted_amt1 = 966; // 1000 energy & 100_000_000 farm tokens
    let first_total1 = first_base_farm_amt + first_boosted_amt1;

    let first_receveived_reward_amt1 =
        farm_setup.claim_rewards(&first_user, 3, first_farm_token_amount);
    assert_eq!(first_receveived_reward_amt1, first_total1);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            6,
            &rust_biguint!(first_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt1),
    );

    // second user claim
    let second_base_farm_amt1 = second_farm_token_amount * 7_500 / total_farm_tokens;

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 4000 / 5_000 + 2500 * 2 * 50_000_000 / 150_000_000) / (3 + 2)
    // (6000 + 1666) / (5) = 1533
    let second_boosted_amt1 = 1533; // 4000 energy & 50_000_000 farm tokens
    let second_total1 = second_base_farm_amt1 + second_boosted_amt1;

    let second_receveived_reward_amt1 =
        farm_setup.claim_rewards(&second_user, 4, second_farm_token_amount);
    assert_eq!(second_receveived_reward_amt1, second_total1);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_receveived_reward_amt1),
    );

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(20);

    // random tx on end of week 2, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(13);
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 13, 1);
    farm_setup.set_user_energy(&third_user, 1, 13, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 8, 1, 1);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(30);

    // random tx on end of week 3, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(20);
    farm_setup.set_user_energy(&first_user, 1_000, 20, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 20, 1);
    farm_setup.set_user_energy(&third_user, 1, 20, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 9, 1, 1);

    // advance week
    farm_setup.b_mock.set_block_epoch(22);
    farm_setup.set_user_energy(&first_user, 1_000, 22, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 22, 1);

    // first user claim2
    let first_base_farm_amt = first_farm_token_amount * 15_000 / total_farm_tokens;

    // Boosted yields rewards for 2 weeks ~= 1931
    let first_boosted_amt2 = 1931; // 1000 energy & 100_000_000 farm tokens
    let first_total2 = first_base_farm_amt + first_boosted_amt2;

    let first_receveived_reward_amt2 =
        farm_setup.claim_rewards(&first_user, 6, first_farm_token_amount);
    assert_eq!(first_receveived_reward_amt2, first_total2);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            10,
            &rust_biguint!(first_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt1 + first_receveived_reward_amt2),
    );

    // second user claim2
    let second_base_farm_amt2 = second_farm_token_amount * 15_000 / total_farm_tokens;

    // Boosted yields rewards for 2 weeks ~= 3067
    let second_boosted_amt2 = 3067; // 4000 energy & 50_000_000 farm tokens
    let second_total2 = second_base_farm_amt2 + second_boosted_amt2;

    let second_receveived_reward_amt2 =
        farm_setup.claim_rewards(&second_user, 7, second_farm_token_amount);
    assert_eq!(second_receveived_reward_amt2, second_total2);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            11,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_receveived_reward_amt1 + second_receveived_reward_amt2),
    );

    // current week = 4
    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 1);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 1);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 1);

    farm_setup.check_error_collect_undistributed_boosted_rewards(
        "Current week must be higher than the week offset",
    );

    // advance to week 6
    farm_setup.b_mock.set_block_epoch(36);

    farm_setup.collect_undistributed_boosted_rewards();
    farm_setup.check_undistributed_boosted_rewards(1);
    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 1);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 1);

    // advance to week 8
    farm_setup.b_mock.set_block_epoch(50);

    farm_setup.collect_undistributed_boosted_rewards();
    farm_setup.check_undistributed_boosted_rewards(3);

    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 0);

    // check entries are not empty
    farm_setup
        .b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            assert!(!sc.total_rewards_for_week(1).is_empty());
            assert!(!sc.total_energy_for_week(1).is_empty());

            assert!(!sc.total_rewards_for_week(3).is_empty());
            assert!(!sc.total_energy_for_week(3).is_empty());
        })
        .assert_ok();

    farm_setup.claim_rewards(&second_user, 11, second_farm_token_amount);

    // check 3rd entry was cleared automatically
    // 1st entry was not cleared, as we paused for too long
    farm_setup
        .b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            assert!(!sc.total_rewards_for_week(1).is_empty());
            assert!(!sc.total_energy_for_week(1).is_empty());

            assert!(sc.total_rewards_for_week(3).is_empty());
            assert!(sc.total_energy_for_week(3).is_empty());
        })
        .assert_ok();
}

#[test]
fn farm_enter_with_multiple_farm_token() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    let third_user = farm_setup.third_user.clone();
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
    farm_setup.exit_farm(&third_user, 5, 1, 1);

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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt),
    );

    // second user additional enter farm

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 4000 / 5_000 + 2500 * 2 * 50_000_000 / 150_000_000) / (3 + 2)
    // (6000 + 1666) / (5) = 1533
    let second_boosted_amt = 1533; // 4000 energy & 50_000_000 farm tokens
    let second_farm_token_amount2 = 50_000_000;
    let second_user_enter_farm_reward = farm_setup.enter_farm_with_additional_payment(
        &second_user,
        second_farm_token_amount2,
        4,
        second_farm_token_amount,
    );

    assert_eq!(second_user_enter_farm_reward, second_boosted_amt);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            7,
            &rust_biguint!(second_farm_token_amount + second_farm_token_amount2),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_boosted_amt),
    );
}

#[test]
fn farm_claim_with_minimum_tokens() {
    let _ = DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);
    let third_user = farm_setup.third_user.clone();

    // first user enter farm
    let first_farm_token_amount = 99_900_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 10_000, 2, 1);
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 100_000;
    let second_user = farm_setup.second_user.clone();
    farm_setup.set_user_energy(&second_user, 90_000, 2, 1);
    farm_setup.enter_farm(&second_user, second_farm_token_amount);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 1, first_farm_token_amount);
    let _ = farm_setup.claim_rewards(&second_user, 2, second_farm_token_amount);

    // advance blocks - 100_800 blocks - 100_800 * 1_000 = 100_800_000 total rewards
    // 75_600_000 base farm, 25_200_000 boosted yields
    farm_setup.b_mock.set_block_nonce(100_800);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 10_000, 6, 1);
    farm_setup.set_user_energy(&second_user, 90_000, 6, 1);
    farm_setup.set_user_energy(&third_user, 1, 6, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 5, 1, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 10_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 90_000, 10, 1);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // first user claim - Applies base formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (25_200_000 * 3 * 10_000 / 100_000 + 25_200_000 * 2 * 99_900_000 / 100_000_000) / (3 + 2)
    // (7_560_000 + 50_349_600) / (5) = 11_581_920
    let first_base_farm_amt = first_farm_token_amount * 75_600_000 / total_farm_tokens;
    let first_boosted_amt = 11_581_920;
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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt),
    );

    // second user claim - Applies user base max rewards

    // total boosted rewards = 25_200_000
    // boosted rewards limited to:
    // 10 * 25_200_000 * 100_000 / 100_000_000 = 25_200_000 / 100 =
    // 252_000
    let second_base_farm_amt = second_farm_token_amount * 75_600_000 / total_farm_tokens; // 75_600
    let second_boosted_amt = 252_000;
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

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_receveived_reward_amt),
    );

    // advance to week 6
    farm_setup.b_mock.set_block_epoch(36);
    let total_boosted_yields_rewards = 25_200_000;
    let remaining_boosted_yields_rewards =
        total_boosted_yields_rewards - first_boosted_amt - second_boosted_amt;
    farm_setup.check_undistributed_boosted_rewards(0);
    farm_setup.collect_undistributed_boosted_rewards();
    farm_setup.check_undistributed_boosted_rewards(remaining_boosted_yields_rewards);
    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 0);
}
