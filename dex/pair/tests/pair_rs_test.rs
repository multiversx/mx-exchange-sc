#![allow(deprecated)]

mod pair_setup;
use energy_factory_mock::EnergyFactoryMock;
use fees_collector::FeesCollector;
use multiversx_sc::codec::{self, TopDecode};
use multiversx_sc::{
    api::ManagedTypeApi,
    codec::{
        derive::{NestedDecode, NestedEncode, TopDecode, TopEncode},
        multi_types::OptionalValue,
        top_encode_to_vec_u8,
    },
    imports::StorageMapper,
    storage::{mappers::VecMapper, StorageKey},
    types::{BigUint, EsdtLocalRole, EsdtTokenPayment, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use pair::{
    config::{ConfigModule as _, MAX_PERCENTAGE},
    fee::FeeModule,
    locking_wrapper::LockingWrapperModule,
    pair_actions::swap::SwapModule,
    safe_price::{PriceObservation, Round, SafePriceModule, MAX_OBSERVATIONS},
    safe_price_view::SafePriceViewModule,
    Pair as _,
};
use pair_setup::*;
use simple_lock::{
    locked_token::{LockedTokenAttributes, LockedTokenModule},
    proxy_lp::{LpProxyTokenAttributes, ProxyLpModule},
    SimpleLock,
};

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Debug)]
pub struct OldPriceObservation<M: ManagedTypeApi> {
    pub first_token_reserve_accumulated: BigUint<M>,
    pub second_token_reserve_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
}

#[derive(TopEncode, NestedEncode, Clone, Debug)]
pub struct TimestampPriceObservation<M: ManagedTypeApi> {
    pub first_token_reserve_accumulated: BigUint<M>,
    pub second_token_reserve_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
    pub recording_timestamp: u64,
    pub lp_supply_accumulated: BigUint<M>,
}

#[test]
fn test_pair_setup() {
    let _ = PairSetup::new(pair::contract_obj, router::contract_obj);
}

#[test]
fn test_add_liquidity() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );
}

#[test]
fn test_swap_fixed_input() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 996);
}

#[test]
fn test_swap_fixed_output() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_output(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 96);
}

#[test]
fn test_perfect_swap_fixed_output() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    let token_amount = 1_001_000;

    pair_setup.add_liquidity(
        token_amount,
        1_000_000,
        token_amount,
        1_000_000,
        1_000_000,
        token_amount,
        token_amount,
    );

    pair_setup.swap_fixed_output(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 996, 0);
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        WEGLD_TOKEN_ID,
        &(rust_biguint!(USER_TOTAL_WEGLD_TOKENS - token_amount - 1_000)),
    );
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        MEX_TOKEN_ID,
        &(rust_biguint!(USER_TOTAL_WEGLD_TOKENS - token_amount + 996)),
    );
}

#[test]
fn test_safe_price_observation_decoding() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let old_observation = OldPriceObservation::<DebugApi> {
                    first_token_reserve_accumulated: managed_biguint!(1u64),
                    second_token_reserve_accumulated: managed_biguint!(1u64),
                    weight_accumulated: 1u64,
                    recording_round: 1u64,
                };

                let buffer = top_encode_to_vec_u8(&old_observation).unwrap();

                let mut new_observation = PriceObservation::<DebugApi>::top_decode(buffer).unwrap();
                assert_eq!(
                    new_observation.first_token_reserve_accumulated,
                    managed_biguint!(1u64)
                );
                assert_eq!(
                    new_observation.second_token_reserve_accumulated,
                    managed_biguint!(1u64)
                );
                assert_eq!(new_observation.weight_accumulated, 1u64);
                assert_eq!(new_observation.recording_round, 1u64);
                assert_eq!(new_observation.recording_timestamp, 0u64);
                assert_eq!(
                    new_observation.lp_supply_accumulated,
                    managed_biguint!(0u64)
                );

                let timestamp_observation = TimestampPriceObservation::<DebugApi> {
                    first_token_reserve_accumulated: managed_biguint!(6u64),
                    second_token_reserve_accumulated: managed_biguint!(7u64),
                    weight_accumulated: 8u64,
                    recording_round: 9u64,
                    recording_timestamp: 11_000u64,
                    lp_supply_accumulated: managed_biguint!(10u64),
                };
                let timestamp_buffer = top_encode_to_vec_u8(&timestamp_observation).unwrap();
                let decoded_timestamp =
                    PriceObservation::<DebugApi>::top_decode(timestamp_buffer.clone()).unwrap();
                assert_eq!(decoded_timestamp.recording_timestamp, 11_000u64);
                assert_eq!(
                    decoded_timestamp.lp_supply_accumulated,
                    managed_biguint!(10u64)
                );

                let mut malformed = timestamp_buffer;
                malformed.push(0u8);
                assert!(PriceObservation::<DebugApi>::top_decode(malformed).is_err());

                new_observation.recording_timestamp = 1_000u64;
                new_observation.lp_supply_accumulated = managed_biguint!(2u64);
                sc.price_observations().push(&new_observation);
                let final_observation = sc.price_observations().get(1);
                assert_eq!(final_observation.recording_timestamp, 1_000u64);
                assert_eq!(
                    new_observation.lp_supply_accumulated,
                    final_observation.lp_supply_accumulated
                );
            },
        )
        .assert_ok();
}

#[test]
fn test_safe_price_migration() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    let starting_round = 1000;
    let payment_amount = 1000;
    let mut expected_amount = 996;

    let weight = 10;
    let mut block_round = starting_round + weight;
    pair_setup.set_block_round(block_round);

    let lp_increase = 1_000_000;
    let min_lp_amount = 1_000;
    let mut lp_amount = lp_increase + min_lp_amount;
    pair_setup.add_liquidity(
        lp_increase + min_lp_amount,
        lp_increase,
        lp_increase + min_lp_amount,
        lp_increase,
        lp_increase,
        lp_increase + min_lp_amount,
        lp_increase + min_lp_amount,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_lp_amount(lp_amount);

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_lp_amount(lp_amount);

    block_round += weight;
    expected_amount -= 2;
    pair_setup.set_block_round(block_round);

    // Change LP amount starting block 1030
    let lp_amount_increase = 998_005;
    lp_amount += lp_amount_increase;
    pair_setup.add_liquidity(
        lp_increase,
        lp_increase,
        996_021,
        996_021,
        lp_amount_increase,
        lp_increase,
        996_021,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_lp_amount(lp_amount);

    block_round += weight;
    expected_amount -= 1;
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_lp_amount(lp_amount);

    block_round += weight;
    expected_amount -= 1;
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    pair_setup.check_lp_amount(lp_amount);

    // Check the normal safe price
    let lp_token_amount = 100_000;
    for (start_round, end_round, expected_wegld, expected_mex) in [
        (1011, 1019, 100_099, 99_900),
        (1020, 1030, 100_199, 99_801),
        (1030, 1040, 100_249, 99_751),
    ] {
        pair_setup.check_lp_tokens_safe_price(
            &pair_address,
            start_round,
            end_round,
            lp_token_amount,
            WEGLD_TOKEN_ID,
            expected_wegld,
            MEX_TOKEN_ID,
            expected_mex,
        );
    }

    // Simulate old price observations
    pair_setup.set_price_observation_as_old(1);
    pair_setup.set_price_observation_as_old(2);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();

    for (start_round, end_round, expected_wegld, expected_mex) in [
        (1011, 1019, 50_124, 50_025),
        (1020, 1030, 50_174, 49_975),
        (1030, 1040, 100_249, 99_751),
    ] {
        pair_setup.check_lp_tokens_safe_price(
            &pair_address,
            start_round,
            end_round,
            lp_token_amount,
            WEGLD_TOKEN_ID,
            expected_wegld,
            MEX_TOKEN_ID,
            expected_mex,
        );
    }
}

#[test]
fn test_safe_price() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    let payment_amount = 1000;
    let starting_round = 1000;
    let mut expected_amount = 996;
    let mut weight = 1;
    let mut block_round = starting_round + weight;
    pair_setup.set_block_round(block_round);

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

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        1, // The accumulated weight should be 1, as it is the first element from the list
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    for _ in 0..8 {
        block_round += weight;
        first_token_reserve += payment_amount;
        second_token_reserve -= expected_amount;
        first_token_accumulated += weight * first_token_reserve;
        second_token_accumulated += weight * second_token_reserve;
        expected_amount -= 2;

        pair_setup.set_block_round(block_round);
        pair_setup.swap_fixed_input(
            WEGLD_TOKEN_ID,
            payment_amount,
            MEX_TOKEN_ID,
            900,
            expected_amount,
        );
        check_price_observation_from(
            &mut pair_setup.b_mock,
            &pair_setup.pair_wrapper,
            &pair_address,
            block_round,
            block_round - starting_round,
            first_token_accumulated,
            second_token_accumulated,
        );
    }

    // Skip 3 rounds for linear interpolation
    weight = 3;
    block_round += weight;
    first_token_reserve += payment_amount;
    second_token_reserve -= expected_amount;
    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;
    expected_amount -= 2;
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
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
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
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
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );
    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    for (start_round, end_round, expected_amount) in [
        (1004, 1005, 992),
        (1014, 1015, 976),
        (1004, 1015, 983),
        (1011, 1014, 979),
    ] {
        check_safe_price_from(
            &mut pair_setup.b_mock,
            &pair_setup.pair_wrapper,
            &pair_address,
            start_round,
            end_round,
            WEGLD_TOKEN_ID,
            1_000,
            MEX_TOKEN_ID,
            expected_amount,
        );
    }
}

#[test]
fn test_safe_price_linear_interpolation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    let min_pool_reserve = 1_000;
    let mut weight = 1;
    let mut block_round = weight;

    pair_setup.set_block_round(block_round);
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
    let first_token_payment_amount = 1_000;
    let mut second_token_expected_amount = 29_880;

    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
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

    block_round += weight;
    pair_setup.set_block_round(block_round);
    second_token_expected_amount = 29_820;
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Skip 1000 rounds
    weight = 1_000;
    block_round += weight;
    pair_setup.set_block_round(block_round);
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

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
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
    second_token_expected_amount = 40_495;

    // In the new round (1003), we save the new reserves that impacted the price from ~30 to ~40
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        first_token_payment_amount,
        MEX_TOKEN_ID,
        second_token_expected_amount,
        second_token_expected_amount,
    );

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        block_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    // Check linear interpolation
    // As rounds pass, the safe price should stabilize towards the 40s price range
    let interpolation_check_round_offset = 40;
    pair_setup.set_block_round(1040);
    for (interpolation_round, expected_amount) in [
        (960, 29_880),
        (970, 31_771),
        (980, 34_293),
        (990, 37_012),
        (1000, 39_955),
    ] {
        check_safe_price_from(
            &mut pair_setup.b_mock,
            &pair_setup.pair_wrapper,
            &pair_address,
            interpolation_round,
            interpolation_round + interpolation_check_round_offset,
            WEGLD_TOKEN_ID,
            first_token_payment_amount,
            MEX_TOKEN_ID,
            expected_amount,
        );
    }
}

// The safe price from the first pair is read from the second pair
// The purpose of this test is to see if values are returned from the correct contract
#[test]
fn test_both_legacy_and_new_safe_price_from_other_contract() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    let payment_amount = 1000;
    let starting_round = 1000;
    let mut expected_amount = 996;
    let weight = 1;
    let mut block_round = starting_round + weight;
    pair_setup.set_block_round(block_round);

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

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.pair_wrapper,
        &pair_address,
        block_round,
        1, // The accumulated weight should be 1, as it is the first element from the list
        first_token_accumulated,
        second_token_accumulated,
    );

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    first_token_accumulated += weight * first_token_reserve;
    second_token_accumulated += weight * second_token_reserve;

    check_price_observation_from(
        &mut pair_setup.b_mock,
        &pair_setup.second_pair_wrapper,
        &pair_address,
        block_round,
        block_round - starting_round,
        first_token_accumulated,
        second_token_accumulated,
    );

    for _ in 0..2 {
        block_round += weight;
        first_token_reserve += payment_amount;
        second_token_reserve -= expected_amount;
        first_token_accumulated += weight * first_token_reserve;
        second_token_accumulated += weight * second_token_reserve;
        expected_amount -= 2;

        pair_setup.set_block_round(block_round);
        pair_setup.swap_fixed_input(
            WEGLD_TOKEN_ID,
            payment_amount,
            MEX_TOKEN_ID,
            900,
            expected_amount,
        );
        check_price_observation_from(
            &mut pair_setup.b_mock,
            &pair_setup.second_pair_wrapper,
            &pair_address,
            block_round,
            block_round - starting_round,
            first_token_accumulated,
            second_token_accumulated,
        );
    }

    let first_token_payment_amount = 100;
    let expected_token_payment_amount = 99;
    check_safe_price_from(
        &mut pair_setup.b_mock,
        &pair_setup.second_pair_wrapper,
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

#[test]
fn test_external_view_extrapolates_legacy_observation_by_round() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    pair_setup.set_block_round(25u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                for observation in [
                    PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(10u64),
                        second_token_reserve_accumulated: managed_biguint!(10u64),
                        weight_accumulated: 1u64,
                        recording_round: 10u64,
                        recording_timestamp: 0u64,
                        lp_supply_accumulated: managed_biguint!(0u64),
                    },
                    PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(110u64),
                        second_token_reserve_accumulated: managed_biguint!(210u64),
                        weight_accumulated: 11u64,
                        recording_round: 20u64,
                        recording_timestamp: 0u64,
                        lp_supply_accumulated: managed_biguint!(0u64),
                    },
                ] {
                    sc.price_observations().push(&observation);
                }
                sc.safe_price_current_index().set(2usize);
                sc.safe_price_legacy_cutover().set((25u64, 150_000u64));
                sc.initialize_current_price_observation();

                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(MEX_TOKEN_ID);
                sc.pair_reserve(&first_token_id)
                    .set(managed_biguint!(30u64));
                sc.pair_reserve(&second_token_id)
                    .set(managed_biguint!(10u64));
                sc.lp_token_supply().set(managed_biguint!(200u64));
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.second_pair_wrapper, |sc| {
            let extrapolated =
                sc.get_price_observation_view(managed_address!(&pair_address), 25u64);
            assert_eq!(extrapolated.recording_round, 25u64);
            assert_eq!(extrapolated.recording_timestamp, 150_000u64);
            assert_eq!(extrapolated.weight_accumulated, 96_000u64);
            assert_eq!(
                extrapolated.first_token_reserve_accumulated,
                managed_biguint!(1_560_000u64)
            );
            assert_eq!(
                extrapolated.second_token_reserve_accumulated,
                managed_biguint!(1_560_000u64)
            );
            assert_eq!(
                extrapolated.lp_supply_accumulated,
                managed_biguint!(6_000_000u64)
            );

            let quote = sc.get_safe_price(
                managed_address!(&pair_address),
                15u64,
                25u64,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(100u64),
                ),
            );
            assert_eq!(quote.token_identifier, managed_token_id!(MEX_TOKEN_ID));
            assert_eq!(quote.amount, managed_biguint!(75u64));

            let lp_quote = sc
                .get_lp_tokens_safe_price(
                    managed_address!(&pair_address),
                    15u64,
                    25u64,
                    managed_biguint!(150u64),
                )
                .into_tuple();
            assert_eq!(lp_quote.0.amount, managed_biguint!(15u64));
            assert_eq!(lp_quote.1.amount, managed_biguint!(11u64));
        })
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.second_pair_wrapper, |sc| {
            sc.get_safe_price_by_timestamp_range(
                managed_address!(&pair_address),
                1u64,
                2u64,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(100u64),
                ),
            );
        })
        .assert_user_error("The price observation does not exist");
}

#[test]
fn test_safe_price_timestamp_save_interval() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // 10 legacy rounds, expressed as milliseconds.
    pair_setup.set_safe_price_timestamp_save_interval(10u64 * 6_000);

    let payment_amount = 1000u64;
    let starting_round = 1000u64;
    let mut expected_amount = 996;
    let starting_weight = 1;
    let weight = 5;
    let mut block_round = starting_round;
    pair_setup.set_block_round(block_round);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // After first swap with 10-round interval, no observation is finalized yet
    // The data is accumulated in the intermediate observation
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // No finalized observations yet
            assert_eq!(sc.price_observations().len(), 0);
            // But intermediate observation should exist
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.weight_accumulated, starting_weight * 6_000);
        })
        .assert_ok();

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // Still accumulating in the latest observation.
    // No finalization yet because the elapsed duration is below the configured interval.
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // Still no finalized observations
            assert_eq!(sc.price_observations().len(), 0);
            // Intermediate continues accumulating
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(
                intermediate.weight_accumulated,
                (starting_weight + weight) * 6_000
            );
        })
        .assert_ok();

    block_round += weight;
    expected_amount -= 2;
    pair_setup.set_block_round(block_round);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // With accumulated weight >= interval (11 >= 10), finalization happens
    // even without a prior finalized observation
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // One finalized observation
            assert_eq!(sc.price_observations().len(), 1);
            // Check the finalized observation
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, block_round);
            assert_eq!(
                observation.weight_accumulated,
                (starting_weight + 2 * weight) * 6_000
            );
        })
        .assert_ok();
}

#[test]
fn test_safe_price_new_timestamp_logic() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // 10 legacy rounds, expressed as milliseconds.
    pair_setup.set_safe_price_timestamp_save_interval(10u64 * 6_000);

    let payment_amount = 1000u64;
    let starting_round = 1000u64;
    let mut expected_amount = 996;
    let starting_weight = 1;
    let weight = 10;
    let mut block_round = starting_round;
    pair_setup.set_block_round(block_round);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // After first swap with 10-round interval, no observation is finalized yet
    // (weight = 1 < 10)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.weight_accumulated, starting_weight * 6_000);
        })
        .assert_ok();

    for (observation_index, expected_weight) in [(1usize, 11u64), (2usize, 21u64), (3usize, 31u64)]
    {
        block_round += weight;
        expected_amount -= 2;
        pair_setup.set_block_round(block_round);
        pair_setup.swap_fixed_input(
            WEGLD_TOKEN_ID,
            payment_amount,
            MEX_TOKEN_ID,
            900,
            expected_amount,
        );

        pair_setup
            .b_mock
            .execute_query(&pair_setup.pair_wrapper, |sc| {
                assert_eq!(sc.price_observations().len(), observation_index);
                let observation = sc.price_observations().get(observation_index);
                assert_eq!(observation.weight_accumulated, expected_weight * 6_000);
            })
            .assert_ok();
    }
}

#[test]
fn test_safe_price_uses_millisecond_weights_for_save_interval() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);

    let starting_round = 10_000u64;
    let starting_timestamp_ms = 1_000_000u64;
    pair_setup.set_block_round_and_timestamp(starting_round, starting_timestamp_ms);
    pair_setup.update_safe_price(1_000_000, 1_000_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            let current_observation = sc.current_price_observation().get();
            assert_eq!(observation.weight_accumulated, 6_000u64);
            assert_eq!(observation.recording_timestamp, starting_timestamp_ms);
            assert_eq!(
                current_observation.weight_accumulated,
                observation.weight_accumulated
            );
            assert_eq!(
                current_observation.recording_timestamp,
                observation.recording_timestamp
            );
        })
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(starting_round + 1, starting_timestamp_ms + 600);
    pair_setup.update_safe_price(1_001_000, 999_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            assert!(!sc.current_price_observation().is_empty());
            let current_observation = sc.current_price_observation().get();
            assert_eq!(current_observation.recording_round, starting_round + 1);
            assert_eq!(
                current_observation.recording_timestamp,
                starting_timestamp_ms + 600
            );
            assert_eq!(current_observation.weight_accumulated, 6_600u64);
        })
        .assert_ok();
}

#[test]
fn test_update_safe_price_same_round_and_timestamp_is_noop() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(12_000u64);

    let block_round = 10_000u64;
    let starting_timestamp_ms = 1_000_000u64;
    pair_setup.set_block_round_and_timestamp(block_round, starting_timestamp_ms);
    pair_setup.update_safe_price(1_000_000, 1_000_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(!sc.current_price_observation().is_empty());
            let current_observation = sc.current_price_observation().get();
            assert_eq!(current_observation.recording_round, block_round);
            assert_eq!(
                current_observation.recording_timestamp,
                starting_timestamp_ms
            );
            assert_eq!(current_observation.weight_accumulated, 6_000u64);
            assert_eq!(
                current_observation.first_token_reserve_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
            assert_eq!(
                current_observation.second_token_reserve_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
            assert_eq!(
                current_observation.lp_supply_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
        })
        .assert_ok();

    pair_setup.update_safe_price(1_001_000, 999_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(sc.price_observations().is_empty());
            assert!(!sc.current_price_observation().is_empty());
            let observation = sc.current_price_observation().get();
            assert_eq!(observation.recording_round, block_round);
            assert_eq!(observation.recording_timestamp, starting_timestamp_ms);
            assert_eq!(observation.weight_accumulated, 6_000u64);
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
            assert_eq!(
                observation.second_token_reserve_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
            assert_eq!(
                observation.lp_supply_accumulated,
                managed_biguint!(6_000_000_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_update_safe_price_converts_legacy_finalized_before_save() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut legacy_price_observations =
                    VecMapper::<DebugApi, OldPriceObservation<DebugApi>>::new(StorageKey::new(
                        b"price_observations",
                    ));
                legacy_price_observations.push(&OldPriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                });
                sc.safe_price_current_index().set(1usize);
            },
        )
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(100u64, 1_000_000u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let current_observation = sc.current_price_observation().get();
            assert_eq!(current_observation.recording_round, 90u64);
            assert_eq!(current_observation.recording_timestamp, 940_000u64);
            assert_eq!(current_observation.weight_accumulated, 12_000u64);
            assert_eq!(
                current_observation.first_token_reserve_accumulated,
                managed_biguint!(60_000u64)
            );
            assert_eq!(
                current_observation.second_token_reserve_accumulated,
                managed_biguint!(120_000u64)
            );
        })
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(101u64, 1_000_600u64);
    pair_setup.update_safe_price(100u64, 200u64, 1_000u64);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_current_index().get(), 2usize);

            let observation = sc.price_observations().get(2);
            assert_eq!(observation.recording_round, 101u64);
            assert_eq!(observation.recording_timestamp, 1_000_600u64);
            assert_eq!(observation.weight_accumulated, 72_600u64);
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(6_120_000u64)
            );
            assert_eq!(
                observation.second_token_reserve_accumulated,
                managed_biguint!(12_240_000u64)
            );
            assert_eq!(
                observation.lp_supply_accumulated,
                managed_biguint!(60_600_000u64)
            );
            let current_observation = sc.current_price_observation().get();
            assert_eq!(
                current_observation.recording_timestamp,
                observation.recording_timestamp
            );
            assert_eq!(
                current_observation.weight_accumulated,
                observation.weight_accumulated
            );

            let legacy_observation = sc.price_observations().get(1);
            assert_eq!(legacy_observation.recording_timestamp, 0u64);
        })
        .assert_ok();
}

#[test]
fn test_pair_upgrade_preserves_existing_safe_price_cutover() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.safe_price_legacy_cutover().set((99u64, 900_000u64)),
        )
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(100u64, 1_000_000u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_legacy_cutover().get(), (99u64, 900_000u64));
        })
        .assert_ok();
}

#[test]
fn test_pair_upgrade_retries_safe_price_cutover_after_zero_round_or_timestamp() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                });
                sc.safe_price_current_index().set(1usize);
            },
        )
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(0u64, 1_000_000u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_user_error("Cannot normalize legacy safe price observation");
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(sc.safe_price_legacy_cutover().is_empty());
            assert!(sc.current_price_observation().is_empty());
        })
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(100u64, 0u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_user_error("Cannot normalize legacy safe price observation");
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(sc.safe_price_legacy_cutover().is_empty());
        })
        .assert_ok();

    pair_setup.b_mock.set_block_timestamp_ms(1_000_000u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_legacy_cutover().get(), (100u64, 1_000_000u64));
            let current_observation = sc.current_price_observation().get();
            assert_eq!(current_observation.recording_round, 90u64);
            assert_eq!(current_observation.recording_timestamp, 940_000u64);
            assert_eq!(current_observation.weight_accumulated, 12_000u64);
            let normalized = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
            assert_eq!(normalized.recording_timestamp, 940_000u64);
            assert_eq!(normalized.weight_accumulated, 12_000u64);
            assert_eq!(
                normalized.first_token_reserve_accumulated,
                managed_biguint!(60_000u64)
            );
            assert_eq!(
                normalized.second_token_reserve_accumulated,
                managed_biguint!(120_000u64)
            );
        })
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(101u64, 1_000_600u64);
    pair_setup.update_safe_price(100u64, 200u64, 1_000u64);
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_current_index().get(), 2usize);
            let transitioned = sc.price_observations().get(2usize);
            assert_eq!(transitioned.recording_round, 101u64);
            assert_eq!(transitioned.recording_timestamp, 1_000_600u64);
            assert_eq!(transitioned.weight_accumulated, 72_600u64);
        })
        .assert_ok();
}

#[test]
fn test_all_legacy_timestamp_is_invariant_after_cutover() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(900u64),
                    second_token_reserve_accumulated: managed_biguint!(1_800u64),
                    weight_accumulated: 90u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                });
                sc.safe_price_current_index().set(1usize);
            },
        )
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(100u64, 1_000_000u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();

    for (current_round, current_timestamp) in [(101u64, 1_006_000u64), (102u64, 1_012_000u64)] {
        pair_setup.set_block_round_and_timestamp(current_round, current_timestamp);
        pair_setup
            .b_mock
            .execute_query(&pair_setup.pair_wrapper, |sc| {
                let observation =
                    sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
                assert_eq!(observation.recording_timestamp, 940_000u64);
                assert_eq!(observation.weight_accumulated, 540_000u64);
            })
            .assert_ok();
    }
}

#[test]
fn test_mixed_legacy_timestamp_is_invariant_with_pending_and_finalized_observations() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    pair_setup.set_block_round_and_timestamp(100u64, 1_000_000u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(900u64),
                    second_token_reserve_accumulated: managed_biguint!(1_100u64),
                    weight_accumulated: 90u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                });
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(6_000_000u64),
                    second_token_reserve_accumulated: managed_biguint!(7_800_000u64),
                    weight_accumulated: 600_000u64,
                    recording_round: 100u64,
                    recording_timestamp: 1_000_000u64,
                    lp_supply_accumulated: managed_biguint!(60_000_000u64),
                });
                sc.safe_price_current_index().set(2usize);
                sc.safe_price_legacy_cutover().set((100u64, 1_000_000u64));
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
            assert_eq!(observation.recording_timestamp, 940_000u64);
        })
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.current_price_observation().set(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(6_060_000u64),
                    second_token_reserve_accumulated: managed_biguint!(7_860_000u64),
                    weight_accumulated: 606_000u64,
                    recording_round: 101u64,
                    recording_timestamp: 1_006_000u64,
                    lp_supply_accumulated: managed_biguint!(60_600_000u64),
                });
            },
        )
        .assert_ok();
    pair_setup.set_block_round_and_timestamp(101u64, 1_006_000u64);
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
            assert_eq!(observation.recording_timestamp, 940_000u64);
        })
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(6_060_000u64),
                    second_token_reserve_accumulated: managed_biguint!(7_860_000u64),
                    weight_accumulated: 606_000u64,
                    recording_round: 101u64,
                    recording_timestamp: 1_006_000u64,
                    lp_supply_accumulated: managed_biguint!(60_600_000u64),
                });
                sc.safe_price_current_index().set(3usize);
            },
        )
        .assert_ok();
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
            assert_eq!(observation.recording_timestamp, 940_000u64);
        })
        .assert_ok();
}

#[test]
fn test_mixed_legacy_timestamp_quote_does_not_drift_when_new_observations_are_added() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                for observation in [
                    PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(800u64),
                        second_token_reserve_accumulated: managed_biguint!(800u64),
                        weight_accumulated: 80u64,
                        recording_round: 80u64,
                        recording_timestamp: 0u64,
                        lp_supply_accumulated: managed_biguint!(0u64),
                    },
                    PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(900u64),
                        second_token_reserve_accumulated: managed_biguint!(1_100u64),
                        weight_accumulated: 90u64,
                        recording_round: 90u64,
                        recording_timestamp: 0u64,
                        lp_supply_accumulated: managed_biguint!(0u64),
                    },
                    PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(6_000_000u64),
                        second_token_reserve_accumulated: managed_biguint!(7_800_000u64),
                        weight_accumulated: 600_000u64,
                        recording_round: 100u64,
                        recording_timestamp: 1_000_000u64,
                        lp_supply_accumulated: managed_biguint!(60_000_000u64),
                    },
                ] {
                    sc.price_observations().push(&observation);
                }
                sc.safe_price_current_index().set(3usize);
                sc.safe_price_legacy_cutover().set((100u64, 1_000_000u64));
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let quote = sc.get_safe_price_by_timestamp_range(
                managed_address!(&pair_address),
                880_000u64,
                940_000u64,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(100u64),
                ),
            );
            assert_eq!(quote.amount, managed_biguint!(300u64));
        })
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.current_price_observation().set(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(6_006_000u64),
                    second_token_reserve_accumulated: managed_biguint!(7_806_000u64),
                    weight_accumulated: 600_600u64,
                    recording_round: 101u64,
                    recording_timestamp: 1_000_600u64,
                    lp_supply_accumulated: managed_biguint!(60_060_000u64),
                });
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let quote = sc.get_safe_price_by_timestamp_range(
                managed_address!(&pair_address),
                880_000u64,
                940_000u64,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(100u64),
                ),
            );
            assert_eq!(quote.amount, managed_biguint!(300u64));
        })
        .assert_ok();
}

#[test]
fn test_safe_price_does_not_write_new_observation_at_timestamp_zero() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_block_round_and_timestamp(1u64, 0u64);
    pair_setup.update_safe_price(10u64, 20u64, 100u64);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(sc.price_observations().is_empty());
            assert!(sc.current_price_observation().is_empty());
            assert_eq!(sc.safe_price_current_index().get(), 0usize);
        })
        .assert_ok();

    pair_setup.set_block_round_and_timestamp(1u64, 600u64);
    pair_setup.update_safe_price(10u64, 20u64, 100u64);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation = sc.price_observations().get(1usize);
            let current_observation = sc.current_price_observation().get();
            assert_eq!(observation.recording_timestamp, 600u64);
            assert_eq!(observation.weight_accumulated, 6_000u64);
            assert_eq!(
                current_observation.recording_timestamp,
                observation.recording_timestamp
            );
            assert_eq!(
                current_observation.weight_accumulated,
                observation.weight_accumulated
            );
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(60_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_safe_price_writer_rejects_legacy_observation_without_valid_origin() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_block_round_and_timestamp(101u64, 1_000_600u64);
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                });
                sc.safe_price_current_index().set(1usize);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(100u64),
                    &managed_biguint!(200u64),
                    &managed_biguint!(1_000u64),
                );
            },
        )
        .assert_user_error("Cannot normalize legacy safe price observation");

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.safe_price_legacy_cutover().set((80u64, 1_000_000u64));
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(100u64),
                    &managed_biguint!(200u64),
                    &managed_biguint!(1_000u64),
                );
            },
        )
        .assert_user_error("Cannot normalize legacy safe price observation");
}

#[test]
fn test_safe_price_view_rejects_legacy_observation_without_valid_cutover() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();
    pair_setup.set_block_round(100u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                });
                sc.safe_price_current_index().set(1usize);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
        })
        .assert_user_error("Cannot normalize legacy safe price observation");

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.safe_price_legacy_cutover().set((80u64, 1_000_000u64));
            },
        )
        .assert_ok();
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
        })
        .assert_user_error("Cannot normalize legacy safe price observation");
}

#[test]
fn test_price_observation_view_keeps_positive_timestamp_in_milliseconds() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round_and_timestamp(100u64, 60_540u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                    recording_timestamp: 540u64,
                    lp_supply_accumulated: managed_biguint!(5u64),
                };

                sc.price_observations().push(&observation);
                sc.safe_price_current_index().set(1usize);
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);

            assert_eq!(observation.recording_timestamp, 540u64);
            assert_eq!(observation.recording_round, 90u64);
            assert_eq!(observation.weight_accumulated, 2u64);
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(10u64)
            );
            assert_eq!(
                observation.second_token_reserve_accumulated,
                managed_biguint!(20u64)
            );
            assert_eq!(observation.lp_supply_accumulated, managed_biguint!(5u64));
        })
        .assert_ok();
}

#[test]
fn test_finalized_observation_remains_the_current_observation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    let starting_round = 10_000u64;
    let starting_timestamp_ms = 1_000_000u64;
    pair_setup.set_block_round_and_timestamp(starting_round, starting_timestamp_ms);
    pair_setup.update_safe_price(1_000_000, 1_000_000, 1_000_000);

    pair_setup.set_block_round_and_timestamp(starting_round + 1, starting_timestamp_ms + 6_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let saved_observation = sc.price_observations().get(1);
            let current_observation = sc.current_price_observation().get();
            assert_eq!(
                current_observation.recording_round,
                saved_observation.recording_round
            );
            assert_eq!(
                current_observation.recording_timestamp,
                saved_observation.recording_timestamp
            );
            let target_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), starting_round);
            assert_eq!(
                target_observation.recording_round,
                saved_observation.recording_round
            );
            assert_eq!(
                target_observation.recording_timestamp,
                saved_observation.recording_timestamp
            );
        })
        .assert_ok();
}

#[test]
fn test_safe_price_zero_timestamp_observation_uses_legacy_cutover() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round_and_timestamp(110u64, 1_700_000_060_000u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let legacy_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 2u64,
                    recording_round: 90u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                };
                let timestamped_reference_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(60_000u64),
                    second_token_reserve_accumulated: managed_biguint!(120_000u64),
                    weight_accumulated: 72_000u64,
                    recording_round: 100u64,
                    recording_timestamp: 1_700_000_000_000u64,
                    lp_supply_accumulated: managed_biguint!(30_000u64),
                };

                sc.price_observations().push(&legacy_observation);
                sc.price_observations()
                    .push(&timestamped_reference_observation);
                sc.safe_price_current_index().set(2usize);
                sc.safe_price_legacy_cutover()
                    .set((110u64, 1_700_000_060_000u64));
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let normalized = sc.get_price_observation_view(managed_address!(&pair_address), 90u64);
            assert_eq!(normalized.recording_timestamp, 1_699_999_940_000u64);
            assert_eq!(normalized.weight_accumulated, 12_000u64);
            assert_eq!(
                normalized.first_token_reserve_accumulated,
                managed_biguint!(60_000u64)
            );
            assert_eq!(
                normalized.second_token_reserve_accumulated,
                managed_biguint!(120_000u64)
            );
            assert_eq!(normalized.lp_supply_accumulated, managed_biguint!(0u64));
        })
        .assert_ok();
}

#[test]
fn test_price_observation_view_rejects_round_progress_without_timestamp_progress() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round_and_timestamp(101u64, 1_000_000u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let latest_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(60_000u64),
                    second_token_reserve_accumulated: managed_biguint!(120_000u64),
                    weight_accumulated: 6_000u64,
                    recording_round: 100u64,
                    recording_timestamp: 1_000_000u64,
                    lp_supply_accumulated: managed_biguint!(300_000u64),
                };

                sc.price_observations().push(&latest_observation);
                sc.safe_price_current_index().set(1usize);
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            sc.get_price_observation_view(managed_address!(&pair_address), 101u64);
        })
        .assert_user_error("The price observation does not exist");
}

#[test]
fn test_safe_price_timestamp_offset_uses_current_price_observation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round_and_timestamp(110u64, 660_000u64);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                let finalized_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(600_000u64),
                    second_token_reserve_accumulated: managed_biguint!(1_200_000u64),
                    weight_accumulated: 600_000u64,
                    recording_round: 100u64,
                    recording_timestamp: 600_000u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                };
                let current_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(660_000u64),
                    second_token_reserve_accumulated: managed_biguint!(1_320_000u64),
                    weight_accumulated: 660_000u64,
                    recording_round: 110u64,
                    recording_timestamp: 660_000u64,
                    lp_supply_accumulated: managed_biguint!(0u64),
                };

                sc.price_observations().push(&finalized_observation);
                sc.safe_price_current_index().set(1usize);
                sc.current_price_observation().set(&current_observation);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let target_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), 105u64);
            assert_eq!(target_observation.recording_round, 105u64);
            assert_eq!(target_observation.recording_timestamp, 630_000u64);
            assert_eq!(target_observation.weight_accumulated, 630_000u64);
            assert_eq!(
                target_observation.first_token_reserve_accumulated,
                managed_biguint!(630_000u64)
            );
            assert_eq!(
                target_observation.second_token_reserve_accumulated,
                managed_biguint!(1_260_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_safe_price_wrapped_observations_use_ring_order() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    let current_index = 3usize;
    let max_observations = MAX_OBSERVATIONS as u64;
    let current_round = max_observations + current_index as u64;
    pair_setup.set_block_round(current_round);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                for index in 1..=MAX_OBSERVATIONS {
                    let recording_round = if index <= current_index {
                        max_observations + index as u64
                    } else {
                        index as u64
                    };
                    let weight_accumulated = recording_round * 6_000;
                    let observation = PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(weight_accumulated * 10),
                        second_token_reserve_accumulated: managed_biguint!(weight_accumulated * 20),
                        weight_accumulated,
                        recording_round,
                        recording_timestamp: weight_accumulated,
                        lp_supply_accumulated: managed_biguint!(weight_accumulated * 100),
                    };

                    sc.price_observations().push(&observation);
                }

                sc.safe_price_current_index().set(current_index);
                sc.initialize_current_price_observation();
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let oldest_round = current_index as u64 + 1;
            let oldest_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), oldest_round);
            assert_eq!(oldest_observation.recording_round, oldest_round);

            let old_segment_round = 10u64;
            let old_segment_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), old_segment_round);
            assert_eq!(old_segment_observation.recording_round, old_segment_round);

            let new_segment_round = max_observations + 2;
            let new_segment_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), new_segment_round);
            assert_eq!(new_segment_observation.recording_round, new_segment_round);
        })
        .assert_ok();
}

#[test]
fn test_safe_price_full_legacy_ring_wraps_into_timestamped_observations() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    let max_observations = MAX_OBSERVATIONS as u64;
    let first_new_round = max_observations + 1;
    pair_setup.set_block_round(first_new_round);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                for index in 1..=MAX_OBSERVATIONS {
                    let recording_round = index as u64;
                    let observation = PriceObservation {
                        first_token_reserve_accumulated: managed_biguint!(recording_round * 10),
                        second_token_reserve_accumulated: managed_biguint!(recording_round * 20),
                        weight_accumulated: recording_round,
                        recording_round,
                        recording_timestamp: 0u64,
                        lp_supply_accumulated: managed_biguint!(0u64),
                    };

                    sc.price_observations().push(&observation);
                }

                sc.safe_price_current_index().set(MAX_OBSERVATIONS);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| sc.upgrade(),
        )
        .assert_ok();
    pair_setup.update_safe_price(10u64, 20u64, 100u64);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), MAX_OBSERVATIONS);
            assert_eq!(sc.safe_price_current_index().get(), 1usize);
            let first_replacement = sc.price_observations().get(1);
            assert_eq!(first_replacement.recording_round, first_new_round);
            assert_eq!(
                first_replacement.recording_timestamp,
                first_new_round * 6_000
            );
            assert!(first_replacement.weight_accumulated > max_observations);
            let current_observation = sc.current_price_observation().get();
            assert_eq!(
                current_observation.recording_timestamp,
                first_replacement.recording_timestamp
            );
            assert_eq!(
                current_observation.weight_accumulated,
                first_replacement.weight_accumulated
            );

            let still_legacy = sc.price_observations().get(2);
            assert_eq!(still_legacy.recording_timestamp, 0u64);
        })
        .assert_ok();

    let second_new_round = first_new_round + 1;
    pair_setup.set_block_round(second_new_round);
    pair_setup.update_safe_price(11u64, 21u64, 100u64);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_current_index().get(), 2usize);
            let second_replacement = sc.price_observations().get(2);
            assert_eq!(second_replacement.recording_round, second_new_round);
            assert_eq!(
                second_replacement.recording_timestamp,
                second_new_round * 6_000
            );

            let inferred_old_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), 3u64);
            assert_eq!(inferred_old_observation.recording_round, 3u64);
            assert_eq!(inferred_old_observation.recording_timestamp, 3u64 * 6_000);
            assert_eq!(inferred_old_observation.weight_accumulated, 3u64 * 6_000);
            assert_eq!(
                inferred_old_observation.first_token_reserve_accumulated,
                managed_biguint!(180_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_locked_asset() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

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

    DebugApi::dummy();
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
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

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
    DebugApi::dummy();

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
    let transfers = [
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
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let fees_collector_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&pair_setup.owner_address),
        fees_collector::contract_obj,
        "fees collector path",
    );

    let energy_factory_mock_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_biguint!(0),
        Some(&pair_setup.owner_address),
        energy_factory_mock::contract_obj,
        "energy factory mock",
    );
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &energy_factory_mock_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.init();
                sc.base_asset_token_id()
                    .set(managed_token_id!(MEX_TOKEN_ID));
                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            },
        )
        .assert_ok();

    let energy_factory_mock_addr = energy_factory_mock_wrapper.address_ref().clone();
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &fees_collector_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.init(
                    managed_address!(&energy_factory_mock_addr),
                    managed_address!(&energy_factory_mock_addr), // unused
                    0,
                    MultiValueEncoded::new(),
                );
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
}

#[test]
fn test_intermediate_observation_finalization() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(3u64 * 6_000);

    let starting_round = 2000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup.update_safe_price(1_000_000, 1_000_000, 1_000_000);

    pair_setup.set_block_round(starting_round + 1);
    pair_setup.update_safe_price(1_001_000, 999_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.weight_accumulated, 12_000u64);
        })
        .assert_ok();

    pair_setup.set_block_round(starting_round + 3);
    pair_setup.update_safe_price(1_002_000, 998_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round + 3);
            assert_eq!(observation.weight_accumulated, 24_000u64);
            assert_eq!(sc.safe_price_current_index().get(), 1);
        })
        .assert_ok();
}

#[test]
fn test_one_legacy_round_timestamp_interval_saves_immediately() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);

    let starting_round = 3000u64;
    pair_setup.set_block_round(starting_round);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.update_safe_price(1_000_000, 1_000_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round);
            assert_eq!(observation.weight_accumulated, 6_000u64);
            let current_observation = sc.current_price_observation().get();
            assert_eq!(
                current_observation.recording_timestamp,
                observation.recording_timestamp
            );
            assert_eq!(
                current_observation.weight_accumulated,
                observation.weight_accumulated
            );
        })
        .assert_ok();

    pair_setup.set_block_round(starting_round + 1);
    pair_setup.update_safe_price(1_001_000, 999_000, 1_000_000);

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 2);
            let second_observation = sc.price_observations().get(2);
            assert_eq!(second_observation.recording_round, starting_round + 1);
        })
        .assert_ok();
}

#[test]
fn test_intermediate_observation_with_zero_reserves() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_block_round(4000u64);
    for (first_token_reserve, second_token_reserve, lp_supply) in [
        (0, 1_000_000, 1_000_000),
        (1_000_000, 0, 1_000_000),
        (1_000_000, 1_000_000, 0),
    ] {
        pair_setup.update_safe_price(first_token_reserve, second_token_reserve, lp_supply);
    }

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(sc.current_price_observation().is_empty());
        })
        .assert_ok();
}
