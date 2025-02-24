#![allow(deprecated)]

mod mex_governance_setup;

use mex_governance::config::ConfigModule;
use mex_governance_setup::*;
use multiversx_sc::imports::MultiValue2;
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint};
use week_timekeeping::WeekTimekeepingModule;

#[test]
fn init_gov_test() {
    let _ = GovSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );
}

#[test]
fn test_whitelist_and_vote_single_farm() {
    // setup
    let mut gov_setup = GovSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let farm_wm = gov_setup.farm_wm_wrapper.address_ref().clone();

    // Set user energy (needed for voting)
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1);

    // Vote for farm
    let votes = vec![MultiValue2::from((farm_wm.clone(), 1_000u64))];
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Verify vote was recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();

            // Check farm is in voted farms list
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wm));
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
fn test_single_user_vote_all_farms() {
    // setup
    let mut gov_setup = GovSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let farm_wm = gov_setup.farm_wm_wrapper.address_ref().clone();
    let farm_wu = gov_setup.farm_wu_wrapper.address_ref().clone();
    let farm_wh = gov_setup.farm_wh_wrapper.address_ref().clone();

    // Set user energy - will split it between 3 farms
    let total_energy = 3_000u64;
    gov_setup.set_user_energy(first_user.clone(), total_energy, 1, total_energy);

    // Vote for all farms - 1000 energy each
    let votes = vec![
        MultiValue2::from((farm_wm.clone(), 1_000u64)),
        MultiValue2::from((farm_wu.clone(), 1_000u64)),
        MultiValue2::from((farm_wh.clone(), 1_000u64)),
    ];
    gov_setup.vote(first_user.clone(), votes).assert_ok();

    // Verify votes were recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();
            let next_week = current_week + 1;

            // Check all farms are in voted farms list
            let farm_wm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wm));
            let farm_wu_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wu));
            let farm_wh_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wh));

            assert!(sc.voted_farms_for_week(next_week).contains(&farm_wm_id));
            assert!(sc.voted_farms_for_week(next_week).contains(&farm_wu_id));
            assert!(sc.voted_farms_for_week(next_week).contains(&farm_wh_id));

            // Check individual vote amounts
            assert_eq!(
                sc.farm_votes_for_week(farm_wm_id, next_week).get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.farm_votes_for_week(farm_wu_id, next_week).get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.farm_votes_for_week(farm_wh_id, next_week).get(),
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
    // setup
    let mut gov_setup = GovSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_wm = gov_setup.farm_wm_wrapper.address_ref().clone();
    let farm_wu = gov_setup.farm_wu_wrapper.address_ref().clone();
    let farm_wh = gov_setup.farm_wh_wrapper.address_ref().clone();

    // Set up energy for all users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000);

    // First user votes for first two farms
    let first_user_votes = vec![
        MultiValue2::from((farm_wm.clone(), 500u64)),
        MultiValue2::from((farm_wu.clone(), 500u64)),
    ];
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();

    // Second user votes for WEGLD-MEX and WEGLD-HTM
    let second_user_votes = vec![
        MultiValue2::from((farm_wm.clone(), 500u64)),
        MultiValue2::from((farm_wh.clone(), 500u64)),
    ];
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Third user votes only for WEGLD-HTM
    let third_user_votes = vec![MultiValue2::from((farm_wh.clone(), 1_000u64))];
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Verify votes were recorded correctly
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let current_week = sc.get_current_week();
            let next_week = current_week + 1;

            let farm_wm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wm));
            let farm_wu_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wu));
            let farm_wh_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wh));

            // Check vote amounts for each farm
            assert_eq!(
                sc.farm_votes_for_week(farm_wm_id, next_week).get(),
                managed_biguint!(1_000)
            ); // 500 from each of first two users
            assert_eq!(
                sc.farm_votes_for_week(farm_wu_id, next_week).get(),
                managed_biguint!(500)
            ); // 500 from first user
            assert_eq!(
                sc.farm_votes_for_week(farm_wh_id, next_week).get(),
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
    // setup
    let mut gov_setup = GovSetup::new(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_wm = gov_setup.farm_wm_wrapper.address_ref().clone();
    let farm_wu = gov_setup.farm_wu_wrapper.address_ref().clone();
    let farm_wh = gov_setup.farm_wh_wrapper.address_ref().clone();

    // Add MEX tokens to owner for incentives
    gov_setup
        .b_mock
        .set_esdt_balance(&owner, MEX_TOKEN_ID, &rust_biguint!(1_000_000));

    // Set up energy for all users
    gov_setup.set_user_energy(first_user.clone(), 2_000, 1, 2_000); // votes for WM, WU
    gov_setup.set_user_energy(second_user.clone(), 2_000, 1, 2_000); // votes for WM, WH
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000); // votes only for WH

    // First user votes for WEGLD-MEX and WEGLD-USDC
    let first_user_votes = vec![
        MultiValue2::from((farm_wm.clone(), 1_000u64)),
        MultiValue2::from((farm_wu.clone(), 1_000u64)),
    ];
    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();

    // Second user votes for WEGLD-MEX and WEGLD-HTM
    let second_user_votes = vec![
        MultiValue2::from((farm_wm.clone(), 1_000u64)),
        MultiValue2::from((farm_wh.clone(), 1_000u64)),
    ];
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Third user votes entirely for WEGLD-HTM farm
    let third_user_votes = vec![MultiValue2::from((farm_wh.clone(), 1_000u64))];
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_ok();

    // Owner adds incentives for the WEGLD-HTM farm
    let incentive_amount = 10_000u64;
    gov_setup
        .incentivize_farm(farm_wh.clone(), owner.clone(), incentive_amount, 2)
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
    // First user should have 0 (didn't vote for HTM farm)
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
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wh));

            // Check total HTM farm votes (should be 2000 - 1000 each from users 2 and 3)
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
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let farm_wm = gov_setup.farm_wm_wrapper.address_ref().clone();
    let farm_wu = gov_setup.farm_wu_wrapper.address_ref().clone();

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);

    // Users vote in week 1
    let first_user_votes = vec![MultiValue2::from((farm_wm.clone(), 1_000u64))];
    let second_user_votes = vec![MultiValue2::from((farm_wu.clone(), 1_000u64))];

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
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        mex_governance::contract_obj,
    );

    let owner = gov_setup.owner.clone();
    let first_user = gov_setup.first_user.clone();
    let second_user = gov_setup.second_user.clone();
    let third_user = gov_setup.third_user.clone();
    let farm_wh = gov_setup.farm_wh_wrapper.address_ref().clone();

    // Add MEX tokens for incentives
    gov_setup
        .b_mock
        .set_esdt_balance(&owner, MEX_TOKEN_ID, &rust_biguint!(1_000_000));

    // Set up energy for users
    gov_setup.set_user_energy(first_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(second_user.clone(), 1_000, 1, 1_000);
    gov_setup.set_user_energy(third_user.clone(), 1_000, 1, 1_000);

    // Users vote for the farm that will be blacklisted
    let first_user_votes = vec![MultiValue2::from((farm_wh.clone(), 1_000u64))];
    let second_user_votes = vec![MultiValue2::from((farm_wh.clone(), 1_000u64))];
    let third_user_votes = vec![MultiValue2::from((farm_wh.clone(), 1_000u64))];

    gov_setup
        .vote(first_user.clone(), first_user_votes)
        .assert_ok();
    gov_setup
        .vote(second_user.clone(), second_user_votes)
        .assert_ok();

    // Add incentives for the farm
    let incentive_amount = 10_000u64;
    gov_setup
        .incentivize_farm(farm_wh.clone(), owner.clone(), incentive_amount, 2)
        .assert_ok();

    // Owner blacklists the farm
    gov_setup.blacklist_farm(farm_wh.clone()).assert_ok();

    // Verify the blacklisted state
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let farm_id = sc.farm_ids().get_id_non_zero(&managed_address!(&farm_wh));

            // Check farm is blacklisted
            assert!(sc.blacklisted_farms().contains(&farm_id));
            // Check farm is not in whitelist
            assert!(!sc.whitelisted_farms().contains(&farm_id));
        })
        .assert_ok();

    // Try to vote for blacklisted farm - should fail
    gov_setup
        .vote(third_user.clone(), third_user_votes)
        .assert_user_error("Farm is blacklisted");
}
