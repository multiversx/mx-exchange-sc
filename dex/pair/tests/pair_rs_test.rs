#![allow(deprecated)]

mod pair_setup;
use fees_collector::{
    config::ConfigModule, fees_accumulation::FeesAccumulationModule, FeesCollector,
};
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
// use pair::safe_price::MAX_OBSERVATIONS;
use pair::{config::MAX_PERCENTAGE, fee::FeeModule, locking_wrapper::LockingWrapperModule, Pair};
use pair_setup::*;
use simple_lock::{
    locked_token::{LockedTokenAttributes, LockedTokenModule},
    proxy_lp::{LpProxyTokenAttributes, ProxyLpModule},
    SimpleLock,
};

#[test]
fn test_pair_setup() {
    let _ = PairSetup::new(pair::contract_obj);
}

#[test]
fn test_add_liquidity() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );
}

#[test]
fn test_swap_fixed_input() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 996);
}

#[test]
fn test_swap_fixed_output() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_output(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 96);
}

#[test]
fn test_safe_price() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    let payment_amount = 1000;
    let starting_round = 1000;
    let mut expected_amount = 996;
    let mut weight = 1;
    let mut block_round = starting_round + weight;
    pair_setup.b_mock.set_block_round(block_round);

    let mut first_token_reserve = 1_002_000;
    let mut second_token_reserve = 1_000_004;
    let mut first_token_accumulated = 1_001_000;
    let mut second_token_accumulated = 1_001_000;
    pair_setup.add_liquidity(
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        first_token_accumulated,
        second_token_accumulated,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        1, // The accumulated weight should be 1, as it is the first element from the list
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Skip 3 rounds for linear interpolation
    weight = 3;
    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    weight = 1;
    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Check first 2 price observations
    expected_amount = 992;
    pair_setup.check_safe_price(
        &pair_address,
        1004,
        1005,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        expected_amount,
    );

    // Check last 2 price observations
    expected_amount = 976;
    pair_setup.check_safe_price(
        &pair_address,
        1014,
        1015,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        expected_amount,
    );

    // Check first and last price observations
    expected_amount = 983;
    pair_setup.check_safe_price(
        &pair_address,
        1004,
        1015,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        expected_amount,
    );

    // Check the interpolation algorithm
    expected_amount = 979;
    pair_setup.check_safe_price(
        &pair_address,
        1011,
        1014,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        expected_amount,
    );
}

#[test]
fn test_safe_price_linear_interpolation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    let min_pool_reserve = 1_000;
    let mut weight = 1;
    let mut block_round = weight;

    pair_setup.b_mock.set_block_round(block_round);
    let mut first_token_reserve = 1_001_000;
    let mut second_token_reserve = 30_030_000;
    let mut first_token_accumulated = weight * first_token_reserve;
    let mut second_token_accumulated = weight * second_token_reserve;

    pair_setup.add_liquidity(
        first_token_reserve,
        first_token_reserve,
        second_token_reserve,
        first_token_reserve,
        first_token_reserve - min_pool_reserve,
        first_token_reserve,
        second_token_reserve,
    );

    // Initial price ~ 30
    let mut first_token_payment_amount = 1_000;
    let mut second_token_expected_amount = 29_880;

    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    first_token_reserve += first_token_payment_amount;
    second_token_reserve -= second_token_expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    weight = 1;
    block_round += weight;
    pair_setup.b_mock.set_block_round(block_round);
    second_token_expected_amount = 29_820;
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Skip 1000 rounds
    weight = 1_000;
    block_round += weight;
    pair_setup.b_mock.set_block_round(block_round);
    first_token_reserve += first_token_payment_amount;
    second_token_reserve -= second_token_expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    let second_token_payment_amount = 5_000_000;
    let first_token_expected_amount = 143_038;

    // First swap in the block after 1000 rounds, we save the reserves from the previous round (round 2)
    pair_setup.swap_fixed_input(
        MEX_TOKEN_ID,
        second_token_payment_amount,
        WEGLD_TOKEN_ID,
        first_token_expected_amount,
        first_token_expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    weight = 1;
    block_round += weight;
    first_token_reserve -= first_token_expected_amount;
    second_token_reserve += second_token_payment_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    // New price ~ 40
    first_token_payment_amount = 1_000;
    second_token_expected_amount = 40_495;

    // In the new round (1003), we save the new reserves that impacted the price from ~30 to ~40
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Check linear interpolation
    // As rounds pass, the safe price should stabilize towards the 40s price range
    let mut interpolation_round = 960;
    let interpolation_check_round_offset = 40;
    pair_setup.b_mock.set_block_round(1040);
    let mut safe_price_expected_amount = 29_880;
    pair_setup.check_safe_price(
        &pair_address,
        interpolation_round,
        interpolation_round + interpolation_check_round_offset,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        safe_price_expected_amount,
    );

    interpolation_round += 10;
    safe_price_expected_amount = 31_771;
    pair_setup.check_safe_price(
        &pair_address,
        interpolation_round,
        interpolation_round + interpolation_check_round_offset,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        safe_price_expected_amount,
    );

    interpolation_round += 10;
    safe_price_expected_amount = 34_293;
    pair_setup.check_safe_price(
        &pair_address,
        interpolation_round,
        interpolation_round + interpolation_check_round_offset,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        safe_price_expected_amount,
    );

    interpolation_round += 10;
    safe_price_expected_amount = 37_012;
    pair_setup.check_safe_price(
        &pair_address,
        interpolation_round,
        interpolation_round + interpolation_check_round_offset,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        safe_price_expected_amount,
    );

    interpolation_round += 10;
    safe_price_expected_amount = 39_955;
    pair_setup.check_safe_price(
        &pair_address,
        interpolation_round,
        interpolation_round + interpolation_check_round_offset,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        safe_price_expected_amount,
    );
}

// The safe price from the first pair is read from the second pair
// The purpose of this test is to see if values are returned from the correct contract
#[test]
fn test_both_legacy_and_new_safe_price_from_other_contract() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    let payment_amount = 1000;
    let starting_round = 1000;
    let mut expected_amount = 996;
    let weight = 1;
    let mut block_round = starting_round + weight;
    pair_setup.b_mock.set_block_round(block_round);

    let mut first_token_reserve = 1_002_000;
    let mut second_token_reserve = 1_000_004;
    let mut first_token_accumulated = 1_001_000;
    let mut second_token_accumulated = 1_001_000;
    pair_setup.add_liquidity(
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        first_token_accumulated,
        second_token_accumulated,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    pair_setup.check_price_observation(
        &pair_address,
        block_round,
        1, // The accumulated weight should be 1, as it is the first element from the list
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    pair_setup.check_price_observation_from_second_pair(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation_from_second_pair(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.b_mock.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_price_observation_from_second_pair(
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    let first_token_payment_amount = 100;
    let expected_token_payment_amount = 99;
    pair_setup.check_safe_price_from_second_pair(
        &pair_address,
        starting_round + 1,
        block_round,
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        expected_token_payment_amount,
    );

    // Check legacy endpoint
    // Should be the same as the result from the new get_safe_price view
    pair_setup.check_safe_price_from_legacy_endpoint(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        expected_token_payment_amount,
    );
}

// Test is commented as it needs a variable change in order to run succesfully
// In order to run the test with the current setup, MAX_OBSERVATIONS const must be set to 100
// This is necessary as using the MAINNET variable requires too many operations for a unit test
// #[test]
// fn test_safe_price_max_length() {
//     let mut pair_setup = PairSetup::new(pair::contract_obj);
//     let pair_address = pair_setup.pair_wrapper.address_ref().clone();
//     let max_observations = MAX_OBSERVATIONS.try_into().unwrap(); // should be 100
//     let min_pool_reserve = 1_000;
//     let weight = 1;
//     let mut block_round = 0u64;

//     let mut first_token_reserve = 1_001_000;
//     let mut second_token_reserve = 30_030_000;
//     let mut first_token_accumulated = weight * first_token_reserve;
//     let mut second_token_accumulated = weight * second_token_reserve;

//     pair_setup.add_liquidity(
//         first_token_reserve,
//         first_token_reserve,
//         second_token_reserve,
//         first_token_reserve,
//         first_token_reserve - min_pool_reserve,
//         first_token_reserve,
//         second_token_reserve,
//     );

//     let mut first_token_payment_amount = 1;
//     let mut second_token_expected_amount = 29;

//     while block_round <= max_observations {
//         // println!("Round: {}", (block_round));

//         block_round += weight;
//         pair_setup.b_mock.set_block_round(block_round);
//         pair_setup.swap_fixed_input(
//             WEGLD_TOKEN_ID,
//             first_token_payment_amount,
//             MEX_TOKEN_ID,
//             1,
//             second_token_expected_amount,
//         );

//         first_token_reserve += first_token_payment_amount;
//         second_token_reserve -= second_token_expected_amount;
//         first_token_accumulated += weight * first_token_reserve;
//         second_token_accumulated += weight * second_token_reserve;

//         second_token_expected_amount = second_token_reserve / first_token_reserve;
//     }

//     let mut second_token_payment_amount = 1_000_000;
//     let mut first_token_expected_amount = 32_171;

//     // Price change
//     block_round += weight;
//     println!("Price change round: {}", (block_round));
//     pair_setup.b_mock.set_block_round(block_round);
//     pair_setup.swap_fixed_input(
//         MEX_TOKEN_ID,
//         second_token_payment_amount,
//         WEGLD_TOKEN_ID,
//         1,
//         first_token_expected_amount,
//     );

//     pair_setup.check_price_observation(
//         &pair_address,
//         block_round,
//         block_round,
//         first_token_accumulated,
//         second_token_accumulated,
//     );

//     first_token_reserve -= first_token_expected_amount;
//     second_token_reserve += second_token_payment_amount;
//     first_token_accumulated += weight * first_token_reserve;
//     second_token_accumulated += weight * second_token_reserve;

//     second_token_payment_amount = 1_000;
//     first_token_expected_amount =
//         second_token_payment_amount * first_token_reserve / second_token_reserve;

//     // Save 10 more price observations, at the beginning of the list
//     while block_round % max_observations <= 10 {
//         // println!("Round: {}", (block_round));

//         block_round += weight;
//         pair_setup.b_mock.set_block_round(block_round);
//         pair_setup.swap_fixed_input(
//             MEX_TOKEN_ID,
//             second_token_payment_amount,
//             WEGLD_TOKEN_ID,
//             1,
//             first_token_expected_amount,
//         );

//         first_token_reserve -= first_token_expected_amount;
//         second_token_reserve += second_token_payment_amount;
//         first_token_accumulated += weight * first_token_reserve;
//         second_token_accumulated += weight * second_token_reserve;

//         first_token_expected_amount =
//             second_token_payment_amount * first_token_reserve / second_token_reserve;
//     }

//     first_token_payment_amount = 1_000;

//     let mut safe_price_rounds_offset = 20;
//     let mut safe_price_expected_amount = 30_894;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_rounds_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );

//     safe_price_rounds_offset = 10;
//     safe_price_expected_amount = 31_820;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_rounds_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );

//     safe_price_rounds_offset = 1;
//     safe_price_expected_amount = 32_038;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_rounds_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );
// }

#[test]
fn test_locked_asset() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // init locking SC
    let rust_zero = rust_biguint!(0);
    let locking_owner = pair_setup.b_mock.create_user_account(&rust_zero);
    let locking_sc_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_zero,
        Some(&locking_owner),
        simple_lock::contract_obj,
        "Some path",
    );

    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    pair_setup.b_mock.set_block_epoch(4);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_locking_sc_address(managed_address!(locking_sc_wrapper.address_ref()));
                sc.set_locking_deadline_epoch(5);
                sc.set_unlock_epoch(10);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(1_000),
            |sc| {
                let ret = sc.swap_tokens_fixed_input(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_biguint!(10),
                );

                assert_eq!(ret.token_identifier, managed_token_id!(LOCKED_TOKEN_ID));
                assert_eq!(ret.token_nonce, 1);
                assert_eq!(ret.amount, managed_biguint!(996));
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(996),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );

    let user_wegld_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, WEGLD_TOKEN_ID, 0);

    // try unlock too early
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(996),
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_user_error("Cannot unlock yet");

    // unlock ok
    pair_setup.b_mock.set_block_epoch(20);

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(996),
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        WEGLD_TOKEN_ID,
        &(user_wegld_balance_before + rust_biguint!(996)),
    );
}

#[test]
fn add_liquidity_through_simple_lock_proxy() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // init locking SC
    let lp_address = pair_setup.pair_wrapper.address_ref().clone();
    let rust_zero = rust_biguint!(0);
    let locking_owner = pair_setup.b_mock.create_user_account(&rust_zero);
    let locking_sc_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_zero,
        Some(&locking_owner),
        simple_lock::contract_obj,
        "Some path",
    );

    // setup locked token
    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.add_lp_to_whitelist(
                managed_address!(&lp_address),
                managed_token_id!(WEGLD_TOKEN_ID),
                managed_token_id!(MEX_TOKEN_ID),
            );
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // setup lp proxy token
    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LP_PROXY_TOKEN_ID));
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    pair_setup.b_mock.set_block_epoch(5);
    let _ = DebugApi::dummy();

    // lock some tokens first
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            WEGLD_TOKEN_ID,
            0,
            &rust_biguint!(1_000_000),
            |sc| {
                sc.lock_tokens_endpoint(10, OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(2_000_000),
            |sc| {
                sc.lock_tokens_endpoint(15, OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(2_000_000),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 15,
        }),
    );

    pair_setup.b_mock.set_block_epoch(5);

    // add liquidity through simple-lock SC - one locked (WEGLD) token, one unlocked (MEX)
    let transfers = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(500_000),
        },
        TxTokenTransfer {
            token_identifier: MEX_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(500_000),
        },
    ];

    pair_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            &transfers[..],
            |sc| {
                let (dust_first_token, dust_second_token, lp_proxy_payment) = sc
                    .add_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    dust_first_token.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(dust_first_token.token_nonce, 0);
                assert_eq!(dust_first_token.amount, managed_biguint!(0));

                assert_eq!(
                    dust_second_token.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(dust_second_token.token_nonce, 0);
                assert_eq!(dust_second_token.amount, managed_biguint!(0));

                assert_eq!(
                    lp_proxy_payment.token_identifier,
                    managed_token_id!(LP_PROXY_TOKEN_ID)
                );
                assert_eq!(lp_proxy_payment.token_nonce, 1);
                assert_eq!(lp_proxy_payment.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LP_PROXY_TOKEN_ID,
        1,
        &rust_biguint!(500_000),
        Some(&LpProxyTokenAttributes::<DebugApi> {
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
            first_token_locked_nonce: 1,
            second_token_id: managed_token_id!(MEX_TOKEN_ID),
            second_token_locked_nonce: 0,
        }),
    );
    pair_setup.b_mock.check_esdt_balance(
        locking_sc_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(500_000),
    );

    let user_locked_token_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, LOCKED_TOKEN_ID, 1);
    let user_mex_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, MEX_TOKEN_ID, 0);

    // remove liquidity
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LP_PROXY_TOKEN_ID,
            1,
            &rust_biguint!(500_000),
            |sc| {
                let (first_payment_result, second_payment_result) = sc
                    .remove_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    first_payment_result.token_identifier,
                    managed_token_id!(LOCKED_TOKEN_ID)
                );
                assert_eq!(first_payment_result.token_nonce, 1);
                assert_eq!(first_payment_result.amount, managed_biguint!(500_000));

                assert_eq!(
                    second_payment_result.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(second_payment_result.token_nonce, 0);
                assert_eq!(second_payment_result.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &(user_locked_token_balance_before + 500_000u32),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        MEX_TOKEN_ID,
        &(user_mex_balance_before + 500_000u32),
    );

    // Add liquidity - same token pair as before -> same nonce (1)
    pair_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            &transfers[..],
            |sc| {
                let (_, _, lp_proxy_payment) = sc
                    .add_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    lp_proxy_payment.token_identifier,
                    managed_token_id!(LP_PROXY_TOKEN_ID)
                );
                assert_eq!(lp_proxy_payment.token_nonce, 1);
                assert_eq!(lp_proxy_payment.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    // test auto-unlock for tokens on remove liquidity
    pair_setup.b_mock.set_block_epoch(30);

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LP_PROXY_TOKEN_ID,
            1,
            &rust_biguint!(500_000),
            |sc| {
                let (first_payment_result, second_payment_result) = sc
                    .remove_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    first_payment_result.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(first_payment_result.token_nonce, 0);
                assert_eq!(first_payment_result.amount, managed_biguint!(500_000));

                assert_eq!(
                    second_payment_result.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(second_payment_result.token_nonce, 0);
                assert_eq!(second_payment_result.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&locking_sc_wrapper, |sc| {
            assert_eq!(sc.known_liquidity_pools().len(), 1);
            assert!(sc
                .known_liquidity_pools()
                .contains(&managed_address!(&lp_address)));
        })
        .assert_ok();
}

#[test]
fn fees_collector_pair_test() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);
    let fees_collector_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_biguint!(0),
        None,
        fees_collector::contract_obj,
        "fees collector path",
    );

    let pair_addr = pair_setup.pair_wrapper.address_ref().clone();
    let energy_factory_mock_addr = pair_setup.pair_wrapper.address_ref().clone();
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &fees_collector_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.init(
                    managed_token_id!(LOCKED_TOKEN_ID),
                    managed_address!(&energy_factory_mock_addr),
                );
                let _ = sc.known_contracts().insert(managed_address!(&pair_addr));

                let mut tokens = MultiValueEncoded::new();
                tokens.push(managed_token_id!(WEGLD_TOKEN_ID));
                tokens.push(managed_token_id!(MEX_TOKEN_ID));

                sc.add_known_tokens(tokens);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.setup_fees_collector(
                    managed_address!(fees_collector_wrapper.address_ref()),
                    MAX_PERCENTAGE / 2,
                );
            },
        )
        .assert_ok();

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 100_000, MEX_TOKEN_ID, 900, 90_669);

    pair_setup.b_mock.check_esdt_balance(
        fees_collector_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(25),
    );

    pair_setup
        .b_mock
        .execute_query(&fees_collector_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(WEGLD_TOKEN_ID))
                    .get(),
                managed_biguint!(25)
            );
        })
        .assert_ok();
}
