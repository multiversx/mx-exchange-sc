#![allow(deprecated)]

pub mod farm_staking_setup;
use farm_staking::{
    claim_stake_farm_rewards::ClaimStakeFarmRewardsModule, stake_farm::StakeFarmModule,
    unstake_farm::UnstakeFarmModule,
};
use farm_staking_setup::*;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc_scenario::{managed_biguint, rust_biguint, DebugApi};

#[test]
fn farm_staking_with_energy_setup_test() {
    let mut fs_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
}

#[test]
fn farm_staking_boosted_rewards_no_energy_test() {
    let _ = DebugApi::dummy();
    let mut fs_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    fs_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
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
fn farm_staking_boosted_rewards_with_energy_test() {
    let _ = DebugApi::dummy();
    let mut fs_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    fs_setup.set_boosted_yields_factors();
    fs_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

    fs_setup.set_user_energy(&fs_setup.user_address.clone(), 10_000, 0, 10);

    let farm_in_amount = 100_000_000;
    fs_setup.stake_farm(farm_in_amount, &[], 1, 0, 0);
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
                let _ = sc.unstake_farm(managed_biguint!(10), OptionalValue::None);
            },
        )
        .assert_ok();

    fs_setup.set_block_epoch(8);

    fs_setup.set_user_energy(&fs_setup.user_address.clone(), 10_000, 8, 10);

    // value taken from the "test_unstake_farm" test
    // originally, it was 40, but since 25% of the rewards go to boosted yields
    // rewards are now only 3/4 * 40 = 30
    //
    // 10 reserved for boosted yields -> 30 + 10
    let expected_reward_token_out = 40;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let expected_reward_per_share = 300_000; // from 400_000 -> 300_000
    fs_setup.claim_rewards(
        farm_in_amount,
        2,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        5,
        expected_reward_per_share,
    );
    fs_setup.check_farm_token_supply(farm_in_amount);
}
