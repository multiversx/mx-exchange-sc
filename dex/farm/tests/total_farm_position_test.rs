#![allow(deprecated)]

mod farm_setup;

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use farm_setup::multi_user_farm_setup::{
    MultiUserFarmSetup, NonceAmountPair, BOOSTED_YIELDS_PERCENTAGE, MAX_PERCENTAGE,
    PER_BLOCK_REWARD_AMOUNT,
};
use multiversx_sc::types::EsdtLocalRole;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, DebugApi,
};
use sc_whitelist_module::SCWhitelistModule;

use crate::farm_setup::multi_user_farm_setup::{FARMING_TOKEN_ID, FARM_TOKEN_ID, REWARD_TOKEN_ID};

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

    // User who called the claim function should not receive anything
    farm_setup
        .b_mock
        .check_esdt_balance(&third_user, REWARD_TOKEN_ID, &rust_biguint!(0));

    // Check allow external claim is set to false
    farm_setup.allow_external_claim_rewards(&first_user, false);

    farm_setup.claim_boosted_rewards_for_user_expect_error(&first_user, &third_user);
}

#[test]
fn total_farm_position_claim_for_other_test() {
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

    // Second user has the same amount of reward tokens
    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_received_reward_amt),
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

    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount * 2);

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
fn farm_total_position_on_claim_migration_test() {
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
    let farm_in_amount = 50_000_000;
    let first_user = farm_setup.first_user.clone();
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

    // claim rewards with both positions
    let first_payment_amount = farm_in_amount / 2;
    let second_payment_amount = farm_in_amount / 4 * 3;
    let total_farm_amount = farm_in_amount * 2;
    let total_farm_position = farm_in_amount + first_payment_amount; // only the first is migrated by being an old position
    let total_claim_payment = first_payment_amount + second_payment_amount;

    let payments = vec![
        NonceAmountPair {
            nonce: 1,
            amount: first_payment_amount,
        },
        NonceAmountPair {
            nonce: 2,
            amount: second_payment_amount,
        },
    ];

    let block_nonce = 10;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount);
    let _ = farm_setup.claim_rewards_with_multiple_payments(&first_user, payments);
    farm_setup.check_user_total_farm_position(&first_user, total_farm_position);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            3,
            &rust_biguint!(total_claim_payment),
            None,
        );

    // User receives rewards only for the new position
    let expected_user_rewards = block_nonce
        * PER_BLOCK_REWARD_AMOUNT
        * (MAX_PERCENTAGE - BOOSTED_YIELDS_PERCENTAGE)
        * first_payment_amount
        / total_farm_amount
        / MAX_PERCENTAGE;
    farm_setup.b_mock.check_esdt_balance(
        &first_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(expected_user_rewards),
    );
}

#[test]
fn farm_total_position_on_merge_migration_test() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // user has 2 old farm position
    let farm_in_amount = 25_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.enter_farm(&first_user, farm_in_amount);

    // Remove current farm position from storage
    farm_setup.set_user_total_farm_position(&first_user, 0);
    farm_setup.check_user_total_farm_position(&first_user, 0);

    // User enters farm again, with 2 new positions
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.enter_farm(&first_user, farm_in_amount);
    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount * 2);

    // Set farm position migration nonce
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farm_position_migration_nonce().set(3);
            },
        )
        .assert_ok();

    let total_farm_tokens = farm_in_amount * 4;
    farm_setup.check_farm_token_supply(total_farm_tokens);

    // merge all 4 farm positions
    let first_payment_amount = farm_in_amount / 2;
    let second_payment_amount = farm_in_amount / 4 * 3;
    let third_payment_amount = farm_in_amount / 2;
    let forth_payment_amount = farm_in_amount / 4;
    let total_payment_amount =
        first_payment_amount + second_payment_amount + third_payment_amount + forth_payment_amount;
    let total_user_position = farm_in_amount * 2 + first_payment_amount + second_payment_amount;
    let payments = vec![
        NonceAmountPair {
            nonce: 1,
            amount: first_payment_amount,
        },
        NonceAmountPair {
            nonce: 2,
            amount: second_payment_amount,
        },
        NonceAmountPair {
            nonce: 3,
            amount: third_payment_amount,
        },
        NonceAmountPair {
            nonce: 4,
            amount: forth_payment_amount,
        },
    ];

    let block_nonce = 10;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    farm_setup.check_user_total_farm_position(&first_user, farm_in_amount * 2); // last 2 positions
    farm_setup.merge_farm_tokens(&first_user, payments);
    farm_setup.check_user_total_farm_position(&first_user, total_user_position);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &first_user,
            FARM_TOKEN_ID,
            5,
            &rust_biguint!(total_payment_amount),
            None,
        );

    farm_setup
        .b_mock
        .check_esdt_balance(&first_user, REWARD_TOKEN_ID, &rust_biguint!(0));
}

#[test]
fn no_boosted_rewards_penalty_for_no_energy_test() {
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
    farm_setup.b_mock.set_block_epoch(13);

    // User unlocks XMEX and has no energy
    farm_setup.set_user_energy(&first_user, 0, 13, 1);

    // random tx on end of the week, to cummulate rewards
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

#[test]
fn total_farm_position_owner_change_test() {
    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    // first user enters farm 6 times
    let farm_token_amount = 10_000_000;
    let half_token_amount = farm_token_amount / 2;
    let first_user = farm_setup.first_user.clone();
    let second_user = farm_setup.second_user.clone();
    let third_user = farm_setup.third_user.clone();

    farm_setup.set_user_energy(&first_user, 1_000, 2, 1);
    farm_setup.enter_farm(&first_user, farm_token_amount);
    farm_setup.enter_farm(&first_user, farm_token_amount);
    farm_setup.enter_farm(&first_user, farm_token_amount);
    farm_setup.enter_farm(&first_user, farm_token_amount);
    farm_setup.enter_farm(&first_user, farm_token_amount);
    farm_setup.enter_farm(&first_user, farm_token_amount);

    let mut first_user_total_position = farm_token_amount * 6;
    let mut second_user_total_position = 0;
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    assert_eq!(farm_setup.last_farm_token_nonce, 6);

    // First user transfers 5 position to second user
    farm_setup.send_farm_position(&first_user, &second_user, 1, farm_token_amount, 0, 2);
    farm_setup.send_farm_position(&first_user, &second_user, 2, farm_token_amount, 0, 2);
    farm_setup.send_farm_position(&first_user, &second_user, 3, farm_token_amount, 0, 2);
    farm_setup.send_farm_position(&first_user, &second_user, 4, farm_token_amount, 0, 2);
    farm_setup.send_farm_position(&first_user, &second_user, 5, farm_token_amount, 0, 2);

    // Total farm position unchanged as users only transfered the farm positions
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    // second user enter farm with LP token + 50% the position from another user
    farm_setup.set_user_energy(&second_user, 4_000, 2, 1);
    farm_setup.enter_farm_with_additional_payment(
        &second_user,
        farm_token_amount,
        1,
        half_token_amount,
    );

    // 1 half farm position was removed from first user and added to the second user (who entered the farm with a position of his own)
    first_user_total_position -= half_token_amount;
    second_user_total_position += farm_token_amount + half_token_amount;
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&first_user, 6, farm_token_amount);
    let _ = farm_setup.claim_rewards(&second_user, 7, farm_token_amount);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_nonce(10);
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&first_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 6, 1);
    farm_setup.set_user_energy(&third_user, 1, 6, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 10, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&first_user, 1_000, 10, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 10, 1);

    // Second user claims with half a position from the first user
    let base_rewards_amount = 535;
    let boosted_rewards_amount = 1414;
    let mut second_user_reward_balance = base_rewards_amount + boosted_rewards_amount;

    let second_received_reward_amt = farm_setup.claim_rewards(&second_user, 2, half_token_amount);
    assert_eq!(second_received_reward_amt, second_user_reward_balance);

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_user_reward_balance),
    );
    farm_setup.b_mock.check_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        11,
        &rust_biguint!(half_token_amount),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(107142857),
            entering_epoch: 2,
            compounded_reward: managed_biguint!(0),
            current_farm_amount: managed_biguint!(half_token_amount),
            original_owner: managed_address!(&second_user),
        }),
    );

    // Check users positions after claim
    first_user_total_position -= half_token_amount;
    second_user_total_position += half_token_amount;
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    // random tx on end of week 2, to cummulate rewards
    farm_setup.b_mock.set_block_nonce(20);
    farm_setup.b_mock.set_block_epoch(13);
    farm_setup.set_user_energy(&first_user, 1_000, 13, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 13, 1);
    farm_setup.set_user_energy(&third_user, 1, 13, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 12, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(15);
    farm_setup.set_user_energy(&first_user, 1_000, 15, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 15, 1);

    // Second user exits farm with half of a position previously owned by user 1
    second_user_reward_balance += 1071; // base rewards
    second_user_reward_balance += 1487; // boosted rewards
    farm_setup.exit_farm(&second_user, 3, half_token_amount);
    farm_setup
        .b_mock
        .check_esdt_balance(&second_user, REWARD_TOKEN_ID, &rust_biguint!(4507));

    // Check users positions after exit
    first_user_total_position -= half_token_amount;
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    // random tx on end of week 3, to cummulate rewards
    farm_setup.b_mock.set_block_nonce(30);
    farm_setup.b_mock.set_block_epoch(20);
    farm_setup.set_user_energy(&first_user, 1_000, 20, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 20, 1);
    farm_setup.set_user_energy(&third_user, 1, 20, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 13, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(25);
    farm_setup.set_user_energy(&first_user, 1_000, 25, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 25, 1);

    // First user claims rewards
    let first_user_received_reward_amt =
        farm_setup.claim_rewards(&first_user, 8, farm_token_amount);
    assert_eq!(first_user_received_reward_amt, 6167);

    // Check users positions after first user claim
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);

    // Second user merges half from one of his original position with 2 position halves from the first user
    let farm_tokens = vec![
        NonceAmountPair {
            nonce: 4,
            amount: half_token_amount,
        },
        NonceAmountPair {
            nonce: 5,
            amount: half_token_amount,
        },
        NonceAmountPair {
            nonce: 11,
            amount: half_token_amount,
        },
    ];

    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_user_reward_balance),
    );
    farm_setup.merge_farm_tokens(&second_user, farm_tokens);
    second_user_reward_balance += 1510; // boosted rewards
    farm_setup.b_mock.check_esdt_balance(
        &second_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(second_user_reward_balance),
    );
    farm_setup.b_mock.check_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        15,
        &rust_biguint!(half_token_amount * 3),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(35714286),
            entering_epoch: 2,
            compounded_reward: managed_biguint!(0),
            current_farm_amount: managed_biguint!(half_token_amount * 3),
            original_owner: managed_address!(&second_user),
        }),
    );

    // Check users positions after merge
    first_user_total_position -= 2 * half_token_amount;
    second_user_total_position += 2 * half_token_amount;
    farm_setup.check_user_total_farm_position(&first_user, first_user_total_position);
    farm_setup.check_user_total_farm_position(&second_user, second_user_total_position);
}

#[test]
fn total_farm_position_through_simple_lock_test() {
    use multiversx_sc::storage::mappers::StorageTokenWrapper;
    use simple_lock::locked_token::LockedTokenModule;
    use simple_lock::proxy_farm::ProxyFarmModule;
    use simple_lock::proxy_farm::*;
    use simple_lock::proxy_lp::{LpProxyTokenAttributes, ProxyLpModule};
    use simple_lock::SimpleLock;

    const LOCKED_TOKEN_ID: &[u8] = b"NOOOO-123456";
    const LOCKED_LP_TOKEN_ID: &[u8] = b"LKLP-123456";
    const FARM_PROXY_TOKEN_ID: &[u8] = b"PROXY-123456";
    const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
    const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID

    DebugApi::dummy();
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );
    let rust_zero = rust_biguint!(0);

    // setup simple lock SC
    let lock_wrapper = farm_setup.b_mock.create_sc_account(
        &rust_zero,
        Some(&farm_setup.owner),
        simple_lock::contract_obj,
        "Simple Lock Path",
    );

    let farm_addr = farm_setup.farm_wrapper.address_ref().clone();
    farm_setup
        .b_mock
        .execute_tx(&farm_setup.owner, &lock_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LOCKED_LP_TOKEN_ID));
            sc.farm_proxy_token()
                .set_token_id(managed_token_id!(FARM_PROXY_TOKEN_ID));
            sc.add_farm_to_whitelist(
                managed_address!(&farm_addr),
                managed_token_id!(FARMING_TOKEN_ID),
                FarmType::SimpleFarm,
            );
        })
        .assert_ok();

    // change farming token for farm + whitelist simple lock contract
    farm_setup
        .b_mock
        .execute_tx(
            &farm_setup.owner,
            &farm_setup.farm_wrapper,
            &rust_zero,
            |sc| {
                sc.farming_token_id()
                    .set(&managed_token_id!(FARMING_TOKEN_ID));
                sc.add_sc_address_to_whitelist(managed_address!(lock_wrapper.address_ref()));
            },
        )
        .assert_ok();

    farm_setup.b_mock.set_esdt_local_roles(
        lock_wrapper.address_ref(),
        LOCKED_LP_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    farm_setup.b_mock.set_esdt_local_roles(
        lock_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // user lock tokens
    let user_addr = farm_setup.first_user.clone();

    let lp_proxy_token_attributes: LpProxyTokenAttributes<DebugApi> = LpProxyTokenAttributes {
        lp_token_id: managed_token_id!(FARMING_TOKEN_ID),
        first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
        first_token_locked_nonce: 1,
        second_token_id: managed_token_id!(MEX_TOKEN_ID),
        second_token_locked_nonce: 2,
    };

    farm_setup.b_mock.set_nft_balance(
        &user_addr,
        LOCKED_LP_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000_000),
        &lp_proxy_token_attributes,
    );

    farm_setup.b_mock.set_esdt_balance(
        lock_wrapper.address_ref(),
        FARMING_TOKEN_ID,
        &rust_biguint!(1_000_000_000),
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    farm_setup.b_mock.set_block_epoch(2);

    let temp_user = farm_setup.third_user.clone();

    farm_setup.check_user_total_farm_position(&user_addr, 0);

    // first user enter farm twice (normal & through simple lock contract)
    // enter farm through simple lock contract
    let farm_in_amount = 50_000_000;
    farm_setup.last_farm_token_nonce += 1;
    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            LOCKED_LP_TOKEN_ID,
            1,
            &rust_biguint!(farm_in_amount),
            |sc| {
                let enter_farm_result = sc.enter_farm_locked_token(FarmType::SimpleFarm);
                let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_PROXY_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, farm_setup.last_farm_token_nonce);
                assert_eq!(out_farm_token.amount, managed_biguint!(farm_in_amount));
            },
        )
        .assert_ok();

    farm_setup.b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        1,
        &rust_biguint!(farm_in_amount),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 1,
            farming_token_id: managed_token_id!(FARMING_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );

    farm_setup.check_user_total_farm_position(&user_addr, farm_in_amount);

    // enter farm directly
    farm_setup.set_user_energy(&user_addr, 1_000, 2, 1);
    farm_setup.enter_farm(&user_addr, farm_in_amount);

    farm_setup.b_mock.check_nft_balance(
        &user_addr,
        FARM_TOKEN_ID,
        farm_setup.last_farm_token_nonce,
        &rust_biguint!(farm_in_amount),
        Some(&FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(0),
            compounded_reward: managed_biguint!(0),
            entering_epoch: 2,
            current_farm_amount: managed_biguint!(farm_in_amount),
            original_owner: managed_address!(&user_addr),
        }),
    );

    farm_setup.check_user_total_farm_position(&user_addr, farm_in_amount * 2);
    farm_setup.check_farm_token_supply(farm_in_amount * 2);

    // users claim rewards to get their energy registered
    let _ = farm_setup.claim_rewards(&user_addr, 2, farm_in_amount);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(10);

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    farm_setup.set_user_energy(&user_addr, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 4, 1);

    // advance 1 week
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&user_addr, 1_000, 10, 1);

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

    let first_received_reward_amt = farm_setup.claim_rewards(&user_addr, 3, farm_in_amount);

    // Should be equal to half base generated rewards + full boosted generated rewards
    assert_eq!(first_received_reward_amt, first_total_rewards);

    farm_setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            &user_addr,
            FARM_TOKEN_ID,
            5,
            &rust_biguint!(farm_in_amount),
            None,
        );

    farm_setup.b_mock.check_esdt_balance(
        &user_addr,
        REWARD_TOKEN_ID,
        &rust_biguint!(first_received_reward_amt),
    );
}
