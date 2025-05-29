#![allow(deprecated)]

mod mex_governance_setup;

use config::ConfigModule;
use mex_governance::{
    config::ConfigModule as _, external_interactions::farm_interactions::FarmInteractionsModule,
};
use mex_governance_setup::*;
use multiversx_sc::{imports::MultiValue2, types::MultiValueEncoded};
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint};
use week_timekeeping::WeekTimekeepingModule;

#[test]
fn init_gov_test() {
    let _ = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );
}

#[test]
fn test_whitelist_and_vote_single_farm() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);

    // Set user energy (needed for voting)
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1);

    // Vote for farm
    let votes = vec![MultiValue2::from((farm_0.clone(), 1_000u64))];
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Verify vote was recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();

            // Check farm is in voted farms list
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_0));
            assert!(sc.voted_farms_for_week(current_week + 1).contains(&farm_id));

            // Check vote amount
            let farm_votes = sc.farm_votes_for_week(farm_id, current_week + 1).get();
            assert_eq!(farm_votes, managed_biguint!(1_000));

            // Check total votes
            let total_votes = sc.total_energy_voted(current_week + 1).get();
            assert_eq!(total_votes, managed_biguint!(1_000));
        })
        .assert_ok();
}

#[test]
fn test_rounding_errors_accumulation() {
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        25,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();

    // Use a prime number for total votes to ensure rounding errors
    let user1_energy = 33_334u64;
    let user2_energy = 33_335u64;
    let user3_energy = 33_334u64;
    let total_energy = user1_energy + user2_energy + user3_energy; // 100,003 - Prime number

    gov_setup.set_user_energy(first_user.clone(), user1_energy, 1, user1_energy);
    gov_setup.set_user_energy(second_user.clone(), user2_energy, 1, user2_energy);
    gov_setup.set_user_energy(third_user.clone(), user3_energy, 1, user3_energy);

    // Create an uneven distribution that will cause rounding errors

    // First user votes
    let mut votes = vec![];
    votes.push(MultiValue2::from((gov_setup.get_farm_address(0), 7_919u64)));
    for i in 1..8 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 3_500u64)));
    }
    votes.push(MultiValue2::from((gov_setup.get_farm_address(24), 915u64)));
    // Total: 7919 + (7 * 3500) + 915 = 33,334
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Second user votes
    let mut votes = vec![];
    for i in 8..17 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 3_703u64)));
    }
    votes.push(MultiValue2::from((gov_setup.get_farm_address(24), 8u64)));
    // Total: (9 * 3703) + 8 = 33,335
    gov_setup.vote(second_user.clone(), votes).assert_ok();

    // Third user votes
    let mut votes = vec![];
    for i in 16..24 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 3_750u64)));
    }
    votes.push(MultiValue2::from((
        gov_setup.get_farm_address(24),
        1_334u64,
    )));
    votes.push(MultiValue2::from((gov_setup.get_farm_address(0), 2_000u64)));
    // Total: (8 * 3750) + 1334 + 2000 = 33,334
    gov_setup.vote(third_user.clone(), votes).assert_ok();

    // Set emissions
    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.set_farm_emissions().assert_ok();

    // Calculate total distributed vs emission rate
    let mut total_distributed = 0u64;
    let emission_rate = DEFAULT_EMISSION_RATE; // 10,000

    // Check all farms except the last one
    for i in 0..24 {
        let farm_wrapper = &gov_setup.farm_wrappers[i];
        gov_setup
            .b_mock
            .execute_query(farm_wrapper, |sc| {
                let per_block_rewards = sc.per_block_reward_amount().get();
                total_distributed += per_block_rewards.to_u64().unwrap();
            })
            .assert_ok();
    }

    // Test that rounding errors accumulate to the last farm
    let last_farm_wrapper = &gov_setup.farm_wrappers[24];
    gov_setup
        .b_mock
        .execute_query(last_farm_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            let last_farm_emission = per_block_rewards.to_u64().unwrap();

            // The last farm should get the remainder
            let expected_last_farm = emission_rate - total_distributed;
            assert_eq!(last_farm_emission, expected_last_farm);

            // Total should equal emission rate exactly
            assert_eq!(total_distributed + last_farm_emission, emission_rate);

            // The last farm gets rounding errors, which could be positive or negative
            // Calculate what it should have gotten without rounding adjustment
            let last_farm_votes = 2_257u64; // 915 + 8 + 1334
            let expected_without_rounding = (emission_rate * last_farm_votes) / total_energy;

            // Assert that rounding error exists and is reasonable
            let rounding_error = if last_farm_emission > expected_without_rounding {
                last_farm_emission - expected_without_rounding
            } else {
                expected_without_rounding - last_farm_emission
            };

            // Rounding error should exist (non-zero) due to prime number total
            assert!(
                rounding_error > 0,
                "Should have rounding error with prime total votes"
            );

            // Rounding error should be small (less than number of farms)
            assert!(
                rounding_error < 25,
                "Rounding error should be less than number of farms"
            );

            // The rounding adjustment ensures no tokens are lost
            assert_eq!(total_distributed + last_farm_emission, emission_rate);
        })
        .assert_ok();
}

#[test]
fn test_single_user_vote_all_farms() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);
    let farm_1 = gov_setup.get_farm_address(1);
    let farm_2 = gov_setup.get_farm_address(2);

    // Set user energy - will split it between 3 farms
    let total_energy = 3_000u64;
    gov_setup.set_user_energy(first_user.clone(), total_energy, 1, total_energy);

    // Vote for all farms - 1000 energy each
    let votes = vec![
        MultiValue2::from((farm_0.clone(), 1_000u64)),
        MultiValue2::from((farm_1.clone(), 1_000u64)),
        MultiValue2::from((farm_2.clone(), 1_000u64)),
    ];
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Verify votes were recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();
            let next_week = current_week + 1;

            // Check all farms are in voted farms list
            let farm_0_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_0));
            let farm_1_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_1));
            let farm_2_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_2));

            assert!(sc.voted_farms_for_week(next_week).contains(&farm_0_id));
            assert!(sc.voted_farms_for_week(next_week).contains(&farm_1_id));
            assert!(sc.voted_farms_for_week(next_week).contains(&farm_2_id));

            // Check individual vote amounts
            assert_eq!(
                sc.farm_votes_for_week(farm_0_id, next_week).get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.farm_votes_for_week(farm_1_id, next_week).get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.farm_votes_for_week(farm_2_id, next_week).get(),
                managed_biguint!(1_000)
            );

            // Check total votes
            assert_eq!(
                sc.total_energy_voted(next_week).get(),
                managed_biguint!(total_energy)
            );
        })
        .assert_ok();
}

#[test]
fn test_multiple_users_vote_multiple_farms() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);
    let farm_1 = gov_setup.get_farm_address(1);
    let farm_2 = gov_setup.get_farm_address(2);

    // Set up energy for all users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000);

    // First user votes for first two farms
    let first_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 500u64)),
        MultiValue2::from((farm_1.clone(), 500u64)),
    ];
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();

    // Second user votes for farm 0 and farm 2
    let second_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 500u64)),
        MultiValue2::from((farm_2.clone(), 500u64)),
    ];
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Third user votes only for farm 2
    let third_user_votes = vec![MultiValue2::from((farm_2.clone(), 1_000u64))];
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Verify votes were recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();
            let next_week = current_week + 1;

            let farm_0_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_0));
            let farm_1_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_1));
            let farm_2_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_2));

            // Check vote amounts for each farm
            assert_eq!(
                sc.farm_votes_for_week(farm_0_id, next_week).get(),
                managed_biguint!(1_000)
            ); // 500 from each of first two users
            assert_eq!(
                sc.farm_votes_for_week(farm_1_id, next_week).get(),
                managed_biguint!(500)
            ); // 500 from first user
            assert_eq!(
                sc.farm_votes_for_week(farm_2_id, next_week).get(),
                managed_biguint!(1_500)
            ); // 500 from second user + 1000 from third user

            // Check total votes
            assert_eq!(
                sc.total_energy_voted(next_week).get(),
                managed_biguint!(3_000)
            );
        })
        .assert_ok();
}

#[test]
fn test_incentivize_farm_and_claim_multiple_users() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);
    let farm_1 = gov_setup.get_farm_address(1);
    let farm_2 = gov_setup.get_farm_address(2);

    // Add MEX tokens to owner for incentives
    gov_setup
        .b_mock
        .set_esdt_balance(&owner, MEX_TOKEN_ID, &rust_biguint!(1_000_000));

    // Set up energy for all users
    gov_setup.set_user_energy(first_user.clone(), 2_000, 1, 2_000);
    gov_setup.set_user_energy(second_user.clone(), 2_000, 1, 2_000);
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000);

    // First user votes for farm 0 and farm 1
    let first_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 1_000u64)),
        MultiValue2::from((farm_1.clone(), 1_000u64)),
    ];
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();

    // Second user votes for farm 0 and farm 2
    let second_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 1_000u64)),
        MultiValue2::from((farm_2.clone(), 1_000u64)),
    ];
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Third user votes entirely for farm 2
    let third_user_votes = vec![MultiValue2::from((farm_2.clone(), 1_000u64))];
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Owner adds incentives for farm 2
    let incentive_amount = 10_000u64;
    gov_setup
        .incentivize_farm(farm_2.clone(), owner.clone(), incentive_amount, 2)
        .assert_ok();

    // Advance epochs to week 3
    gov_setup.b_mock.set_block_epoch(15);

    // Users claim incentives
    gov_setup
        .claim_incentives(first_user.clone(), 2)
        .assert_ok();
    gov_setup
        .claim_incentives(second_user.clone(), 2)
        .assert_ok();
    gov_setup
        .claim_incentives(third_user.clone(), 2)
        .assert_ok();

    // Check users received correct incentives
    // First user should have 0 (didn't vote for farm 2)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user, MEX_TOKEN_ID, &rust_biguint!(0));

    // Second and third users should split the incentives proportionally (both voted 1000 each, so 50-50)
    let expected_reward = incentive_amount / 2;
    gov_setup.b_mock.check_esdt_balance(
        &second_user,
        MEX_TOKEN_ID,
        &rust_biguint!(expected_reward),
    );
    gov_setup
        .b_mock
        .check_esdt_balance(&third_user, MEX_TOKEN_ID, &rust_biguint!(expected_reward));

    // Verify total distribution
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_2));

            // Check total farm 2 votes (should be 2000 - 1000 each from users 2 and 3)
            assert_eq!(
                sc.farm_votes_for_week(farm_id, 2).get(),
                managed_biguint!(2_000)
            );
        })
        .assert_ok();
}

#[test]
fn test_change_emission_rate_and_distributions() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);
    let farm_1 = gov_setup.get_farm_address(1);

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);

    // Users vote in week 1
    let first_user_votes = vec![MultiValue2::from((farm_0.clone(), 1_000u64))];
    let second_user_votes = vec![MultiValue2::from((farm_1.clone(), 1_000u64))];

    gov_setup
        .vote(first_user.clone(), first_user_votes.clone())
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes.clone())
        .assert_ok();

    // Owner changes emission rate for next week
    let new_emission_rate = 20_000u64; // Double the default rate
    gov_setup
        .b_mock
        .execute_tx(&owner, &gov_setup.gov_wrapper, &rust_biguint!(0), |sc| {
            sc.set_reference_emission_rate(managed_biguint!(new_emission_rate));
        })
        .assert_ok();

    // Advance epochs to next week
    gov_setup.b_mock.set_block_epoch(10);

    // Users vote again
    gov_setup.set_user_energy(first_user.clone(), 1_000, 20, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 20, 1_000);
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Verify emission rates are updated correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let first_voting_week = 2;
            let current_voting_week = 3;

            // Check old week rate unchanged
            assert_eq!(
                sc.emission_rate_for_week(first_voting_week).get(),
                managed_biguint!(DEFAULT_EMISSION_RATE)
            );

            // Check new week has updated rate
            assert_eq!(
                sc.emission_rate_for_week(current_voting_week).get(),
                managed_biguint!(new_emission_rate)
            );
        })
        .assert_ok();
}

#[test]
fn test_blacklist_farm_with_active_votes() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_2 = gov_setup.get_farm_address(2);

    // Add MEX tokens for incentives
    gov_setup
        .b_mock
        .set_esdt_balance(&owner, MEX_TOKEN_ID, &rust_biguint!(1_000_000));

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000);

    // Users vote for the farm that will be blacklisted
    let first_user_votes = vec![MultiValue2::from((farm_2.clone(), 1_000u64))];
    let second_user_votes = vec![MultiValue2::from((farm_2.clone(), 1_000u64))];
    let third_user_votes = vec![MultiValue2::from((farm_2.clone(), 1_000u64))];

    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Add incentives for the farm
    let incentive_amount = 10_000u64;
    gov_setup
        .incentivize_farm(farm_2.clone(), owner.clone(), incentive_amount, 2)
        .assert_ok();

    // Owner blacklists the farm
    gov_setup.blacklist_farm(farm_2.clone()).assert_ok();

    // Verify the blacklisted state
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_2));

            // Check farm is blacklisted
            assert!(sc.blacklisted_farms().contains(&farm_id));
        })
        .assert_ok();

    // Try to vote for blacklisted farm - should fail
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_user_error("Farm is blacklisted");
}

#[test]
fn test_set_farm_emissions() {
    let mut gov_setup = GovSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let farm_0 = gov_setup.get_farm_address(0);
    let farm_1 = gov_setup.get_farm_address(1);
    let farm_2 = gov_setup.get_farm_address(2);

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 6_000, 1, 6_000);
    gov_setup.set_user_energy(second_user.clone(), 4_000, 1, 4_000);

    // Users vote with different distributions to test proportional allocation
    // First user: 6000 total energy - 3000 to farm 0, 2000 to farm 1, 1000 to farm 2
    let first_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 3_000u64)),
        MultiValue2::from((farm_1.clone(), 2_000u64)),
        MultiValue2::from((farm_2.clone(), 1_000u64)),
    ];

    // Second user: 4000 total energy - 1000 to farm 0, 1000 to farm 1, 2000 to farm 2
    let second_user_votes = vec![
        MultiValue2::from((farm_0.clone(), 1_000u64)),
        MultiValue2::from((farm_1.clone(), 1_000u64)),
        MultiValue2::from((farm_2.clone(), 2_000u64)),
    ];

    // Submit votes
    gov_setup
        .vote(first_user.clone(), first_user_votes.clone())
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes.clone())
        .assert_ok();

    // Advance to next week (votes were for week 2)
    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.b_mock.set_block_nonce(100);

    // Call set_farm_emissions
    gov_setup.set_farm_emissions().assert_ok();

    // Verify each farm has correct per_block_rewards set
    // Total energy voted: 10,000
    // Farm 0: 4,000 votes (40% of total)
    // Farm 1: 3,000 votes (30% of total)
    // Farm 2: 3,000 votes (30% of total)

    // With DEFAULT_EMISSION_RATE = 10,000
    // Expected rewards:
    // Farm 0: 10,000 * 0.4 = 4,000
    // Farm 1: 10,000 * 0.3 = 3,000
    // Farm 2: 10,000 * 0.3 = 3,000

    // Check farm 0 rewards
    let farm_0_wrapper = &gov_setup.farm_wrappers[0];
    gov_setup
        .b_mock
        .execute_query(farm_0_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(4_000));
        })
        .assert_ok();

    // Check farm 1 rewards
    let farm_1_wrapper = &gov_setup.farm_wrappers[1];
    gov_setup
        .b_mock
        .execute_query(farm_1_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(3_000));
        })
        .assert_ok();

    // Check farm 2 rewards
    let farm_2_wrapper = &gov_setup.farm_wrappers[2];
    gov_setup
        .b_mock
        .execute_query(farm_2_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(3_000));
        })
        .assert_ok();

    // Update emission rate for next week
    gov_setup
        .b_mock
        .execute_tx(
            &gov_setup.owner,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_reference_emission_rate(managed_biguint!(20_000));
            },
        )
        .assert_ok();

    // Users vote again with the same distribution
    gov_setup.set_user_energy(first_user.clone(), 6_000, 11, 6_000);
    gov_setup.set_user_energy(second_user.clone(), 4_000, 11, 4_000);

    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Advance to yet another week
    gov_setup.b_mock.set_block_epoch(20);
    gov_setup.b_mock.set_block_nonce(200);

    // Call set_farm_emissions again
    gov_setup.set_farm_emissions().assert_ok();

    // With new emission rate of 20,000, expect doubled rewards
    // Farm 0: 20,000 * 0.4 = 8,000
    // Farm 1: 20,000 * 0.3 = 6,000
    // Farm 2: 20,000 * 0.3 = 6,000

    // Check farm 0 rewards with new rate
    let farm_0_wrapper = &gov_setup.farm_wrappers[0];
    gov_setup
        .b_mock
        .execute_query(farm_0_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(8_000));
        })
        .assert_ok();

    // Check farm 1 rewards with new rate
    let farm_1_wrapper = &gov_setup.farm_wrappers[1];
    gov_setup
        .b_mock
        .execute_query(farm_1_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(6_000));
        })
        .assert_ok();

    // Check farm 2 rewards with new rate
    let farm_2_wrapper = &gov_setup.farm_wrappers[2];
    gov_setup
        .b_mock
        .execute_query(farm_2_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(6_000));
        })
        .assert_ok();
}

#[test]
fn test_edge_case_all_votes_outside_top_25() {
    // Setup with 30 farms
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let fourth_user = gov_setup.b_mock.create_user_account(&rust_biguint!(0));

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(second_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(third_user.clone(), 20_000, 1, 20_000);
    gov_setup.set_user_energy(fourth_user.clone(), 10_000, 1, 10_000);

    // First user votes for farms 0-9 (10 farms max)
    let mut first_user_votes = vec![];
    for i in 0..10 {
        first_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();

    // Second user votes for farms 10-19
    let mut second_user_votes = vec![];
    for i in 10..20 {
        second_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Third user votes for farms 20-24 to complete top 25
    let mut third_user_votes = vec![];
    for i in 20..25 {
        third_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Fourth user votes only for farms 25-29 with small amounts
    // These votes will all be redistributed because farms 25-29 won't make it to top 25
    let mut fourth_user_votes = vec![];
    for i in 25..30 {
        fourth_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 2_000u64)));
    }
    gov_setup
        .vote(fourth_user.clone(), fourth_user_votes)
        .assert_ok();

    // Verify state
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            // Top 25 should be farms 0-24 (each with 4000 votes)
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();
            assert_eq!(farm_emissions.len(), 25, "Should have exactly 25 farms");

            // All of fourth user's votes (10,000 total) should be redistributed
            let redistributed = sc.redistributed_votes_for_week(voting_week).get();
            assert_eq!(redistributed, managed_biguint!(10_000));

            // Verify farms 0-24 are in the list with 4000 votes each
            for emission in farm_emissions.iter() {
                assert_eq!(emission.farm_emission, managed_biguint!(4_000));
            }
        })
        .assert_ok();

    // Advance to next week
    gov_setup.b_mock.set_block_epoch(10);

    // This should now work correctly - no division by zero because top farms have votes
    gov_setup.set_farm_emissions().assert_ok();
}

#[test]
fn test_top_farms_selection_with_redistribution() {
    // Setup with 30 farms to test the top 25 selection
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 35_500, 1, 35_500);
    gov_setup.set_user_energy(second_user.clone(), 34_500, 1, 34_500);
    gov_setup.set_user_energy(third_user.clone(), 27_000, 1, 27_000);

    // First user votes for farms 0-9 with decreasing amounts
    let mut first_user_votes = vec![];
    for i in 0..10 {
        let vote_amount = 4000 - (i as u64 * 100); // 4000, 3900, 3800...
        first_user_votes.push(MultiValue2::from((
            gov_setup.get_farm_address(i),
            vote_amount,
        )));
    }

    // Second user votes for farms 10-19 with decreasing amounts
    let mut second_user_votes = vec![];
    for i in 10..20 {
        let vote_amount = 3900 - ((i - 10) as u64 * 100); // 3900, 3800, 3700...
        second_user_votes.push(MultiValue2::from((
            gov_setup.get_farm_address(i),
            vote_amount,
        )));
    }

    // Third user votes for farms 15-24 (overlap with second user + more)
    let mut third_user_votes = vec![];
    for i in 15..25 {
        let vote_amount = 2700;
        third_user_votes.push(MultiValue2::from((
            gov_setup.get_farm_address(i),
            vote_amount,
        )));
    }

    // Submit votes
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Now add a fourth user to vote for farms 25-29 (outside top 25)
    let fourth_user = gov_setup.b_mock.create_user_account(&rust_biguint!(0));
    gov_setup.set_user_energy(fourth_user.clone(), 25_000, 1, 25_000);

    let mut fourth_user_votes = vec![];
    for i in 25..30 {
        let vote_amount = 5000; // Equal votes for each
        fourth_user_votes.push(MultiValue2::from((
            gov_setup.get_farm_address(i),
            vote_amount,
        )));
    }
    gov_setup
        .vote(fourth_user.clone(), fourth_user_votes)
        .assert_ok();

    // Verify top farms selection and redistribution
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            // Check that only top 25 farms are in the emissions list
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();
            assert_eq!(farm_emissions.len(), 25, "Should have exactly 25 farms");

            // Check redistributed votes
            let redistributed = sc.redistributed_votes_for_week(voting_week).get();

            // Calculate expected redistribution
            // Farms 25-29 have 5000 votes each and make it to top 25
            // Farms 20-24 have 2700 votes each and get redistributed
            assert_eq!(
                redistributed,
                managed_biguint!(13_500),
                "Votes for farms 20-24 (2700 each × 5) should be redistributed"
            );

            // Verify farms are sorted by vote amount in descending order
            for i in 0..farm_emissions.len() - 1 {
                let current = &farm_emissions.get(i);
                let next = &farm_emissions.get(i + 1);
                assert!(
                    current.farm_emission >= next.farm_emission,
                    "Farms should be sorted in descending order"
                );
            }

            // Verify the top farm has the highest votes
            let top_farm = farm_emissions.get(0);
            // Farm 15 should be top with votes from both second user (3400) and third user (2700) = 6100
            assert_eq!(top_farm.farm_emission, managed_biguint!(6_100));

            // Verify total votes calculation
            let total_votes = sc.total_energy_voted(voting_week).get();
            assert_eq!(total_votes, managed_biguint!(122_000)); // 35,500 + 34,500 + 27,000 + 25,000
        })
        .assert_ok();

    // Advance to next week
    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.b_mock.set_block_nonce(100);

    // Set farm emissions
    gov_setup.set_farm_emissions().assert_ok();

    // Get farm addresses for testing before queries
    let farm_0_address = gov_setup.get_farm_address(0);
    let farm_15_address = gov_setup.get_farm_address(15);

    // Verify redistribution was applied correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            // Test farm 0 (has 4000 votes)
            let farm_0_id = sc
                .farm_ids()
                .get_id_non_zero(&managed_address!(&farm_0_address));
            let farm_0_votes = sc.farm_votes_for_week(farm_0_id, 2).get();
            assert_eq!(farm_0_votes, managed_biguint!(4_000));

            // Test farm 15 (top farm with 6100 votes)
            let farm_15_id = sc
                .farm_ids()
                .get_id_non_zero(&managed_address!(&farm_15_address));
            let farm_15_votes = sc.farm_votes_for_week(farm_15_id, 2).get();
            assert_eq!(farm_15_votes, managed_biguint!(6_100));
        })
        .assert_ok();

    // Verify actual farm emissions with redistribution
    let farm_0_wrapper = &gov_setup.farm_wrappers[0];
    gov_setup
        .b_mock
        .execute_query(farm_0_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();

            // Calculate expected emission for farm 0:
            // Base emission: 10,000 * 4,000 / 122,000 ≈ 327
            // Redistribution calculation: 4,000 / 108,500 * 13,500 ≈ 497
            // Redistribution bonus: 10,000 * 497 / 122,000 ≈ 41
            // Total expected: 327 + 41 = 368

            // Due to integer arithmetic, check within reasonable range
            assert!(
                per_block_rewards >= managed_biguint!(360)
                    && per_block_rewards <= managed_biguint!(380),
                "Farm 0 emission should be around 368, got: {:?}",
                per_block_rewards
            );
        })
        .assert_ok();

    // Verify top farm gets higher redistribution
    let farm_15_wrapper = &gov_setup.farm_wrappers[15];
    gov_setup
        .b_mock
        .execute_query(farm_15_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();

            // Farm 15 should get more than farm 0 due to higher vote count
            // Base emission: 10,000 * 6,100 / 122,000 ≈ 500
            // Redistribution calculation: 6,100 / 108,500 * 13,500 ≈ 759
            // Redistribution bonus: 10,000 * 759 / 122,000 ≈ 62
            // Total expected: 500 + 62 = 562

            assert!(
                per_block_rewards >= managed_biguint!(550)
                    && per_block_rewards <= managed_biguint!(570),
                "Farm 15 emission should be around 562, got: {:?}",
                per_block_rewards
            );
        })
        .assert_ok();
}

#[test]
fn test_division_by_zero_when_all_votes_redistributed() {
    // This test demonstrates the division by zero issue
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    gov_setup.set_user_energy(first_user.clone(), 10_000, 1, 10_000);

    // Vote only for farms outside top 25
    let mut votes = vec![];
    for i in 25..30 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 2_000u64)));
    }

    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Advance to next week and set emissions
    gov_setup.b_mock.set_block_epoch(10);

    // This should cause issues because top_farms_total_votes = 0
    // In the current implementation, this would cause a division by zero
    gov_setup.set_farm_emissions().assert_ok();
}

#[test]
fn test_less_than_25_farms_no_redistribution() {
    // Setup with only 10 farms (less than MAX_REWARDED_FARMS)
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        10,
    );

    let first_user = gov_setup.first_user.clone();
    gov_setup.set_user_energy(first_user.clone(), 10_000, 1, 10_000);

    // Vote for all 10 farms
    let mut votes = vec![];
    for i in 0..10 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }

    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Verify no redistribution occurs
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            // All farms should be in top farms
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();
            assert_eq!(farm_emissions.len(), 10);

            // No redistribution should occur
            let redistributed = sc.redistributed_votes_for_week(voting_week).get();
            assert_eq!(redistributed, managed_biguint!(0));
        })
        .assert_ok();

    // Advance and set emissions
    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.set_farm_emissions().assert_ok();

    // Verify each farm gets exact proportional share
    let farm_0_wrapper = &gov_setup.farm_wrappers[0];
    gov_setup
        .b_mock
        .execute_query(farm_0_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            // Each farm has 1000 votes out of 10000 total = 10%
            // 10% of 10000 emission rate = 1000
            assert_eq!(per_block_rewards, managed_biguint!(1_000));
        })
        .assert_ok();
}

#[test]
fn test_farm_ranking_changes_between_weeks() {
    // Test that farm rankings can change between voting weeks
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();

    // Week 1: Set up energy
    gov_setup.set_user_energy(first_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(second_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(third_user.clone(), 20_000, 1, 20_000);

    // Week 1 votes: Farm 25 is outside top 25
    // First user: farms 0-9
    let mut week1_votes_user1 = vec![];
    for i in 0..10 {
        week1_votes_user1.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(first_user.clone(), week1_votes_user1)
        .assert_ok();

    // Second user: farms 10-19
    let mut week1_votes_user2 = vec![];
    for i in 10..20 {
        week1_votes_user2.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(second_user.clone(), week1_votes_user2)
        .assert_ok();

    // Third user: farms 20-24
    let mut week1_votes_user3 = vec![];
    for i in 20..25 {
        week1_votes_user3.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup
        .vote(third_user.clone(), week1_votes_user3)
        .assert_ok();

    // Verify farm 25 is not in top 25
    let farm_25_address = gov_setup.get_farm_address(25);
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();

            // Farm 25 should not be in the list
            let farm_25_managed = managed_address!(&farm_25_address);
            let mut found_farm_25 = false;
            for emission in farm_emissions.iter() {
                if emission.farm_address == farm_25_managed {
                    found_farm_25 = true;
                    break;
                }
            }
            assert!(!found_farm_25, "Farm 25 should not be in top 25 in week 1");
        })
        .assert_ok();

    // Advance to week 2
    gov_setup.b_mock.set_block_epoch(10);

    // Week 2: Different voting pattern - Farm 25 gets massive votes
    gov_setup.set_user_energy(first_user.clone(), 35_000, 10, 35_000);
    gov_setup.set_user_energy(second_user.clone(), 35_000, 10, 35_000);
    gov_setup.set_user_energy(third_user.clone(), 30_000, 10, 30_000);

    // All users vote heavily for farm 25
    let mut week2_votes_user1 = vec![];
    week2_votes_user1.push(MultiValue2::from((farm_25_address.clone(), 30_000u64)));
    for i in 0..5 {
        week2_votes_user1.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup
        .vote(first_user.clone(), week2_votes_user1)
        .assert_ok();

    let mut week2_votes_user2 = vec![];
    week2_votes_user2.push(MultiValue2::from((farm_25_address.clone(), 30_000u64)));
    for i in 5..10 {
        week2_votes_user2.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup
        .vote(second_user.clone(), week2_votes_user2)
        .assert_ok();

    let mut week2_votes_user3 = vec![];
    week2_votes_user3.push(MultiValue2::from((farm_25_address.clone(), 20_000u64)));
    for i in 10..18 {
        week2_votes_user3.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_111u64)));
    }
    // Last farm gets the remainder to make it exactly 30,000
    week2_votes_user3.push(MultiValue2::from((
        gov_setup.get_farm_address(18),
        1_112u64,
    )));
    gov_setup
        .vote(third_user.clone(), week2_votes_user3)
        .assert_ok();

    // Verify farm 25 is now in top 25 (actually should be #1)
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 3;
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();

            // Farm 25 should be first with 80,000 votes
            let first_farm = farm_emissions.get(0);
            let farm_25_managed = managed_address!(&farm_25_address);
            assert_eq!(first_farm.farm_address, farm_25_managed);
            assert_eq!(first_farm.farm_emission, managed_biguint!(80_000));
        })
        .assert_ok();
}

#[test]
fn test_exactly_25_farms_with_votes() {
    // Test when exactly 25 farms receive votes
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();

    gov_setup.set_user_energy(first_user.clone(), 10_000, 1, 10_000);
    gov_setup.set_user_energy(second_user.clone(), 10_000, 1, 10_000);
    gov_setup.set_user_energy(third_user.clone(), 5_000, 1, 5_000);

    // First user votes for farms 0-9
    let mut votes = vec![];
    for i in 0..10 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Second user votes for farms 10-19
    let mut votes = vec![];
    for i in 10..20 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup.vote(second_user.clone(), votes).assert_ok();

    // Third user votes for farms 20-24
    let mut votes = vec![];
    for i in 20..25 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup.vote(third_user.clone(), votes).assert_ok();

    // Verify state
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            // Exactly 25 farms in emissions
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();
            assert_eq!(farm_emissions.len(), 25);

            // No redistribution
            let redistributed = sc.redistributed_votes_for_week(voting_week).get();
            assert_eq!(redistributed, managed_biguint!(0));

            // All farms should have equal votes
            for emission in farm_emissions.iter() {
                assert_eq!(emission.farm_emission, managed_biguint!(1_000));
            }
        })
        .assert_ok();

    // Set emissions
    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.set_farm_emissions().assert_ok();

    // Each farm should get exactly 1/25 of total emissions
    let expected_per_farm = DEFAULT_EMISSION_RATE / 25; // 10,000 / 25 = 400

    let farm_0_wrapper = &gov_setup.farm_wrappers[0];
    gov_setup
        .b_mock
        .execute_query(farm_0_wrapper, |sc| {
            let per_block_rewards = sc.per_block_reward_amount().get();
            assert_eq!(per_block_rewards, managed_biguint!(expected_per_farm));
        })
        .assert_ok();
}

#[test]
fn test_tied_votes_at_boundary() {
    // Test when multiple farms have the same votes around the 25th position
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let fourth_user = gov_setup.b_mock.create_user_account(&rust_biguint!(0));

    gov_setup.set_user_energy(first_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(second_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(third_user.clone(), 10_000, 1, 10_000);
    gov_setup.set_user_energy(fourth_user.clone(), 10_000, 1, 10_000);

    let mut votes = vec![];

    // First user: Farms 0-9: 4000 votes each (40,000 total)
    for i in 0..10 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Second user: Farms 10-19: 4000 votes each (40,000 total)
    let mut votes = vec![];
    for i in 10..20 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }
    gov_setup.vote(second_user.clone(), votes).assert_ok();

    // Third user: Farms 20-29: 1000 votes each (10,000 total) - tied at boundary
    let mut votes = vec![];
    for i in 20..30 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup.vote(third_user.clone(), votes).assert_ok();

    // Fourth user also votes for some of the boundary farms to create more ties
    let mut votes = vec![];
    for i in 18..28 {
        votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 1_000u64)));
    }
    gov_setup.vote(fourth_user.clone(), votes).assert_ok();

    // Verify how ties are handled
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;
            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();

            // Should have 25 farms
            assert_eq!(farm_emissions.len(), 25);

            // Count redistributed votes
            let redistributed = sc.redistributed_votes_for_week(voting_week).get();

            // Verify sorting is consistent
            for i in 0..farm_emissions.len() - 1 {
                let current = &farm_emissions.get(i);
                let next = &farm_emissions.get(i + 1);
                assert!(
                    current.farm_emission >= next.farm_emission,
                    "Farms should be sorted in descending order"
                );
            }

            // Expected votes:
            // Farms 0-17: 4000 votes each
            // Farms 18-19: 5000 votes (4000 + 1000)
            // Farms 20-27: 2000 votes (1000 + 1000)
            // Farms 28-29: 1000 votes

            // After sorting, the order should be:
            // Position 0-1: Farms 18-19 with 5000 votes
            // Position 2-19: Farms 0-17 with 4000 votes
            // Position 20-24: Five farms from 20-27 with 2000 votes

            // Check first two positions (5000 votes)
            for i in 0..2 {
                let emission = farm_emissions.get(i);
                assert_eq!(emission.farm_emission, managed_biguint!(5_000));
            }

            // Check positions 2-19 (4000 votes)
            for i in 2..20 {
                let emission = farm_emissions.get(i);
                assert_eq!(emission.farm_emission, managed_biguint!(4_000));
            }

            // Check positions 20-24 (2000 votes)
            for i in 20..25 {
                let emission = farm_emissions.get(i);
                assert_eq!(emission.farm_emission, managed_biguint!(2_000));
            }

            // Three farms with 2000 votes and two farms with 1000 votes should be outside top 25
            // So redistributed = 3*2000 + 2*1000 = 8000
            assert_eq!(redistributed, managed_biguint!(8_000));
        })
        .assert_ok();
}

#[test]
fn test_redistribution_calculation_accuracy() {
    let mut gov_setup = GovSetup::new_with_farms(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
        30,
    );

    let mut all_farm_addresses = Vec::new();
    for i in 0..30 {
        all_farm_addresses.push(gov_setup.get_farm_address(i));
    }

    gov_setup
        .b_mock
        .execute_tx(
            &gov_setup.owner,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut farm_addresses = MultiValueEncoded::new();
                for farm_address in all_farm_addresses.iter() {
                    farm_addresses.push(managed_address!(farm_address));
                }
                sc.reset_farm_emissions(farm_addresses);
            },
        )
        .assert_ok();

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let fourth_user = gov_setup.b_mock.create_user_account(&rust_biguint!(0));

    gov_setup.set_user_energy(first_user.clone(), 20_000, 1, 20_000);
    gov_setup.set_user_energy(second_user.clone(), 20_000, 1, 20_000);
    gov_setup.set_user_energy(third_user.clone(), 40_000, 1, 40_000);
    gov_setup.set_user_energy(fourth_user.clone(), 20_000, 1, 20_000);

    let mut first_user_votes = vec![];
    for i in 0..10 {
        first_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 2_000u64)));
    }

    let mut second_user_votes = vec![];
    for i in 10..20 {
        second_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 2_000u64)));
    }

    let mut third_user_votes = vec![];
    for i in 20..25 {
        third_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 8_000u64)));
    }

    let mut fourth_user_votes = vec![];
    for i in 25..30 {
        fourth_user_votes.push(MultiValue2::from((gov_setup.get_farm_address(i), 4_000u64)));
    }

    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();
    gov_setup
        .vote(fourth_user.clone(), fourth_user_votes)
        .assert_ok();

    let farm_0_address = gov_setup.get_farm_address(0);
    let farm_20_address = gov_setup.get_farm_address(20);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            let farm_emissions = sc.farm_emissions_for_week(voting_week).get();
            assert_eq!(farm_emissions.len(), 25);

            let redistributed = sc.redistributed_votes_for_week(voting_week).get();
            assert_eq!(redistributed, managed_biguint!(10_000));

            let total_votes = sc.total_energy_voted(voting_week).get();
            assert_eq!(total_votes, managed_biguint!(100_000));
        })
        .assert_ok();

    gov_setup.b_mock.set_block_epoch(10);
    gov_setup.b_mock.set_block_nonce(100);

    gov_setup.set_farm_emissions().assert_ok();

    // Count emissions only from farms where produce_rewards is enabled
    let mut total_distributed = 0u64;
    let mut farms_with_active_rewards = 0u32;

    for i in 0..30 {
        let farm_wrapper = &gov_setup.farm_wrappers[i];
        gov_setup
            .b_mock
            .execute_query(farm_wrapper, |sc| {
                let produce_rewards_enabled = sc.produce_rewards_enabled().get();
                if produce_rewards_enabled {
                    let per_block_rewards = sc.per_block_reward_amount().get();
                    total_distributed += per_block_rewards.to_u64().unwrap();
                    farms_with_active_rewards += 1;
                }
            })
            .assert_ok();
    }

    assert_eq!(
        farms_with_active_rewards, 25,
        "Should have exactly 25 farms with rewards enabled"
    );
    assert_eq!(total_distributed, DEFAULT_EMISSION_RATE);

    let mut farm_0_rewards = 0u64;
    let mut farm_20_rewards = 0u64;

    gov_setup
        .b_mock
        .execute_query(&gov_setup.farm_wrappers[0], |sc| {
            if sc.produce_rewards_enabled().get() {
                farm_0_rewards = sc.per_block_reward_amount().get().to_u64().unwrap();
            }
        })
        .assert_ok();

    gov_setup
        .b_mock
        .execute_query(&gov_setup.farm_wrappers[20], |sc| {
            if sc.produce_rewards_enabled().get() {
                farm_20_rewards = sc.per_block_reward_amount().get().to_u64().unwrap();
            }
        })
        .assert_ok();

    assert!(
        farm_20_rewards > farm_0_rewards,
        "Farm 20 ({} rewards) should get more than farm 0 ({} rewards)",
        farm_20_rewards,
        farm_0_rewards
    );

    let reward_ratio = farm_20_rewards as f64 / farm_0_rewards as f64;
    assert!(
        (3.5..=4.5).contains(&reward_ratio),
        "Reward ratio ({:.2}) should be roughly 4x for 4x vote difference",
        reward_ratio
    );

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let voting_week = 2;

            let farm_0_id = sc
                .farm_ids()
                .get_id_non_zero(&managed_address!(&farm_0_address));
            let farm_20_id = sc
                .farm_ids()
                .get_id_non_zero(&managed_address!(&farm_20_address));

            let farm_0_votes = sc.farm_votes_for_week(farm_0_id, voting_week).get();
            let farm_20_votes = sc.farm_votes_for_week(farm_20_id, voting_week).get();

            assert_eq!(farm_0_votes, managed_biguint!(2_000));
            assert_eq!(farm_20_votes, managed_biguint!(8_000));
        })
        .assert_ok();
}
