#![allow(deprecated)]

use common_structs::FarmTokenAttributes;
use multiversx_sc_scenario::{managed_address, managed_biguint, rust_biguint, DebugApi};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::farm_with_locked_rewards_setup::{
    FarmSetup, BOOSTED_YIELDS_PERCENTAGE, FARM_TOKEN_ID, LOCKED_REWARD_TOKEN_ID,
};

mod farm_with_locked_rewards_setup;

#[test]
fn farm_with_no_boost_no_proxy_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
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
    let _ = DebugApi::dummy();
    let mut farm_setup = FarmSetup::new(
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
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
