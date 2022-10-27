use common_structs::FarmTokenAttributes;
use elrond_wasm_debug::{managed_biguint, rust_biguint, DebugApi};

pub mod farm_setup;
use farm_setup::*;

#[test]
fn farm_with_no_boost_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = FarmSetup::new(farm::contract_obj, energy_factory_mock::contract_obj);

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
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(first_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(first_farm_token_amount),
    };
    let first_rewards_amt = farm_setup.calculate_rewards(
        &first_user,
        first_farm_token_nonce,
        first_farm_token_amount,
        first_attributes,
    );
    let first_expected_rewards_amt = first_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(first_rewards_amt, first_expected_rewards_amt);

    // calculate rewards - second user
    let second_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(second_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(second_farm_token_amount),
    };
    let second_rewards_amt = farm_setup.calculate_rewards(
        &second_user,
        second_farm_token_nonce,
        second_farm_token_amount,
        second_attributes,
    );
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
    let mut farm_setup = FarmSetup::new(farm::contract_obj, energy_factory_mock::contract_obj);

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
    // (6000 + 1666) / (5) = 1532
    let second_boosted_amt = 1532; // 4000 energy & 50_000_000 farm tokens
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
fn farm_known_proxy_test() {
    let _ = DebugApi::dummy();
    let mut farm_setup = FarmSetup::new(farm::contract_obj, energy_factory_mock::contract_obj);

    // first user enter farm
    let first_farm_token_amount = 100_000_000;
    let first_farm_token_nonce = 1u64;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    // second user enter farm
    let second_farm_token_amount = 50_000_000;
    let second_farm_token_nonce = 2u64;
    let second_user = farm_setup.second_user.clone();
    farm_setup.enter_farm(&first_user, second_farm_token_amount);

    farm_setup.add_known_proxy(&first_user);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    farm_setup.b_mock.set_block_nonce(10);

    let total_farm_tokens = first_farm_token_amount + second_farm_token_amount;

    // calculate rewards - first user
    let first_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(first_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(first_farm_token_amount),
    };
    let first_rewards_amt = farm_setup.calculate_rewards(
        &first_user,
        first_farm_token_nonce,
        first_farm_token_amount,
        first_attributes,
    );
    let first_expected_rewards_amt = first_farm_token_amount * 10_000 / total_farm_tokens;
    assert_eq!(first_rewards_amt, first_expected_rewards_amt);

    // calculate rewards - second user
    let second_attributes = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0),
        original_entering_epoch: 0,
        entering_epoch: 0,
        initial_farming_amount: managed_biguint!(second_farm_token_amount),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(second_farm_token_amount),
    };
    let second_rewards_amt = farm_setup.calculate_rewards(
        &second_user,
        second_farm_token_nonce,
        second_farm_token_amount,
        second_attributes,
    );
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
    let mut farm_setup = FarmSetup::new(farm::contract_obj, energy_factory_mock::contract_obj);

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
    // (6000 + 1666) / (5) = 1532
    let second_boosted_amt1 = 1532; // 4000 energy & 50_000_000 farm tokens
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
    farm_setup.exit_farm(&third_user, 8, 1);

    // advance blocks - 10 blocks - 10 * 1_000 = 10_000 total rewards
    // 7_500 base farm, 2_500 boosted yields
    farm_setup.b_mock.set_block_nonce(30);

    // random tx on end of week 3, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(20);
    farm_setup.set_user_energy(&first_user, 1_000, 20, 1);
    farm_setup.set_user_energy(&second_user, 4_000, 20, 1);
    farm_setup.set_user_energy(&third_user, 1, 20, 1);
    farm_setup.enter_farm(&third_user, 1);
    farm_setup.exit_farm(&third_user, 9, 1);

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

    // Boosted yields rewards for 2 weeks ~= 3066
    let second_boosted_amt2 = 3066; // 4000 energy & 50_000_000 farm tokens
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
    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 2);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 2);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 1);

    farm_setup.check_error_collect_undistributed_boosted_rewards(
        "Current week must be higher than the week offset",
    );

    // advance to week 6
    farm_setup.b_mock.set_block_epoch(36);

    farm_setup.collect_undistributed_boosted_rewards();
    farm_setup.check_undistributed_boosted_rewards(2);
    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 2);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 1);

    // advance to week 8
    farm_setup.b_mock.set_block_epoch(50);

    farm_setup.collect_undistributed_boosted_rewards();
    farm_setup.check_undistributed_boosted_rewards(5);

    farm_setup.check_remaining_boosted_rewards_to_distribute(1, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(2, 0);
    farm_setup.check_remaining_boosted_rewards_to_distribute(3, 0);
}
