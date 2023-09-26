#![allow(deprecated)]

mod farm_setup;

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use farm_setup::multi_user_farm_setup::{MultiUserFarmSetup, BOOSTED_YIELDS_PERCENTAGE};
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint, DebugApi};

use crate::farm_setup::multi_user_farm_setup::{FARM_TOKEN_ID, REWARD_TOKEN_ID};

#[test]
fn total_farm_position_claim_test() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
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

    farm_setup.check_farm_token_supply(farm_in_amount * 2);

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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_reward_amt),
    );
}

#[test]
fn allow_external_claim_rewards_setting_test() {
    DebugApi::dummy();
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

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 1, first_farm_token_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(10);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&third_user, 1, 6, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 3, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);

    let first_boosted_amt = 2500;

    // Second user claim boosted rewards for first user
    farm_setup.allow_external_claim_rewards(&first_user, true);

    let first_received_boosted_amt =
        farm_setup.claim_boosted_rewards_for_user(&first_user, &third_user);
    assert_eq!(first_received_boosted_amt, first_boosted_amt);

    // First user should receive the boosted rewards
    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_boosted_amt),
    );

    // Check allow external claim is set to false
    farm_setup.allow_external_claim_rewards(&first_user, false);

    farm_setup.claim_boosted_rewards_for_user_expect_error(&first_user, &third_user);
}

#[test]
fn test_total_farm_position_claim_for_other_test() {
    DebugApi::dummy();
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
    farm_setup.exit_farm(&third_user, 5, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 10, 1);

    // Second user claims for himself
    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;
    let second_base_farm_amt = second_farm_token_amount * 7_500 / total_farm_tokens;
    let second_boosted_amt = 1533; // 4000 energy & 50_000_000 farm tokens
    let second_total = second_base_farm_amt + second_boosted_amt;

    let second_received_reward_amt =
        farm_setup.claim_rewards(&second_user, 4, second_farm_token_amount);
    assert_eq!(second_received_reward_amt, second_total);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &second_user,
            FARM_TOKEN_ID,
            6,
            &rust_biguint!(second_farm_token_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_received_reward_amt),
    );

    // Boosted yields rewards formula
    // total_boosted_rewards * (energy_const * user_energy / total_energy + farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (total_boosted_rewards * energy_const * user_energy / total_energy + total_boosted_rewards * farm_const * user_farm / total_farm) / (energy_const + farm_const)
    // (2500 * 3 * 1_000 / 5_000 + 2500 * 2 * 100_000_000 / 150_000_000) / (3 + 2)
    // (1500 + 3333) / (5) = 966
    let first_boosted_amt = 966; // 1000 energy & 100_000_000 farm tokens

    // Second user claim boosted rewards for first user
    farm_setup.allow_external_claim_rewards(&first_user, true);

    let first_received_boosted_amt =
        farm_setup.claim_boosted_rewards_for_user(&first_user, &second_user);
    assert_eq!(first_received_boosted_amt, first_boosted_amt);

    // First user should receive the boosted rewards
    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_boosted_amt),
    );
}

#[test]
fn farm_total_position_migration_test() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
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

    // Remove current farm position from storage
    farm_setup.set_user_total_farm_position(&first_user, 0);
    farm_setup.check_user_total_farm_position(&first_user, 0);

    // User enters farm again
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount);

    // Set farm position migration nonce
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farm_position_migration_nonce().set(2);
            },
        )
        .assert_ok();

    farm_setup.check_farm_token_supply(farm_in_amount * 2);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 2, farm_in_amount);

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

    let first_boosted_amt = 2_000; // claim boosted with only half total position - not full rewards
    let first_total_rewards = first_base_farm_amt + first_boosted_amt;

    let first_received_reward_amt = farm_setup.claim_rewards(&first_user, 3, farm_in_amount);

    // Should be equal to half base generated rewards + partial boosted generated rewards
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

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_reward_amt),
    );

    // advance 10 more blocks - 10_000 more total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(20);

    // random tx on end of week 2, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(13);
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.set_user_energy(&temp_user, 1, 13, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 6, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(20);
    farm_setup.set_user_energy(&first_user, 1_000, 20, 1);

    // user claims with old position - should migrate his entire position
    let second_received_reward_amt = farm_setup.claim_rewards(&first_user, 1, farm_in_amount);

    let second_base_farm_amt = (farm_in_amount * 7_500 / total_farm_tokens) * 2; // user claims with initial position (2 weeks worth of rewards)
    let second_boosted_amt = 2_500; // claim boosted with entire total position - receives full rewards
    let second_total_rewards = second_base_farm_amt + second_boosted_amt;
    assert_eq!(second_received_reward_amt, second_total_rewards);
}

#[test]
fn farm_total_position_exit_migration_test() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
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

    // Remove current farm position from storage
    farm_setup.set_user_total_farm_position(&first_user, 0);
    farm_setup.check_user_total_farm_position(&first_user, 0);

    // User enters farm again
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount);

    // Set farm position migration nonce
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farm_position_migration_nonce().set(2);
            },
        )
        .assert_ok();

    farm_setup.check_farm_token_supply(farm_in_amount * 2);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 2, farm_in_amount);

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

    // first user exist farm with old position
    farm_setup.exit_farm(&first_user, 1, farm_in_amount);

    // user farm position should be unchanged
    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount);

    // User should receive half base rewards and full boosted rewards
    let total_farm_tokens = farm_in_amount * 2;
    let first_base_farm_amt = farm_in_amount * 7_500 / total_farm_tokens;
    let first_boosted_amt = 2_500;
    let first_total_rewards = first_base_farm_amt + first_boosted_amt;
    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_total_rewards),
    );
}

#[test]
fn no_boosted_rewards_penalty_for_exit_farm_test() {
    DebugApi::dummy();
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(5);

    let temp_user = farm_setup.third_user.clone();

    // first user enter farm
    let farm_in_amount = 50_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.set_user_energy(&first_user, 1_000, 5, 1);
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 2, farm_in_amount);

    // advance to week 1

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(10);

    // random tx on end of the week, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 4, 1);

    // advance to week 2
    farm_setup.b_mock.set_block_nonce(20);

    // random tx on end of the week, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(13);
    farm_setup.set_user_energy(&temp_user, 1, 13, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 5, 1);

    // advance to week 3
    farm_setup.b_mock.set_block_nonce(30);

    // random tx on end of the week, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(20);
    farm_setup.set_user_energy(&temp_user, 1, 20, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 6, 1);

    // advance to week 4
    farm_setup.b_mock.set_block_epoch(25);
    farm_setup.set_user_energy(&first_user, 0, 25, 1);

    // first user claims 3 weeks worth of rewards (2-4)
    let total_farm_tokens = farm_in_amount * 2;
    let first_base_farm_amt = (farm_in_amount * 7_500 / total_farm_tokens) * 3;
    let first_boosted_amt = 2_500 * 3;
    let first_total_rewards = first_base_farm_amt + first_boosted_amt;

    let first_receveived_reward_amt = farm_setup.claim_rewards(&first_user, 1, farm_in_amount);

    // Should be equal to half base generated rewards + full boosted generated rewards
    assert_eq!(first_receveived_reward_amt, first_total_rewards);

    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_receveived_reward_amt),
    );
}
