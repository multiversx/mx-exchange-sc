#![allow(deprecated)]

mod pair_setup;
use energy_factory_mock::EnergyFactoryMock;
use fees_collector::FeesCollector;
use multiversx_sc::codec::{self, TopDecode};
use multiversx_sc::{
    api::ManagedTypeApi,
    codec::{
        derive::{NestedEncode, TopEncode},
        multi_types::OptionalValue,
        top_encode_to_vec_u8,
    },
    types::{BigUint, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use pair::{
    config::MAX_PERCENTAGE,
    fee::FeeModule,
    locking_wrapper::LockingWrapperModule,
    pair_actions::swap::SwapModule,
    safe_price::{PriceObservation, Round, SafePriceModule, MAX_OBSERVATIONS},
    safe_price_view::SafePriceViewModule,
};
use pair_setup::*;
use simple_lock::{
    locked_token::{LockedTokenAttributes, LockedTokenModule},
    proxy_lp::{LpProxyTokenAttributes, ProxyLpModule},
    SimpleLock,
};

#[derive(TopEncode, NestedEncode, Clone, Debug)]
pub struct OldPriceObservation<M: ManagedTypeApi> {
    pub first_token_reserve_accumulated: BigUint<M>,
    pub second_token_reserve_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
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
    let _ = pair_setup.b_mock.execute_tx(
        &pair_setup.owner_address,
        &pair_setup.pair_wrapper,
        &rust_biguint!(0),
        |sc| {
            let old_observation: OldPriceObservation<DebugApi> = OldPriceObservation {
                first_token_reserve_accumulated: managed_biguint!(1u64),
                second_token_reserve_accumulated: managed_biguint!(1u64),
                weight_accumulated: 1u64,
                recording_round: 1u64,
            };

            let buffer = top_encode_to_vec_u8(&old_observation).unwrap();

            let mut new_observation = PriceObservation::<DebugApi>::top_decode(buffer).unwrap();
            assert_eq!(
                new_observation.lp_supply_accumulated,
                managed_biguint!(0u64)
            );

            new_observation.lp_supply_accumulated = managed_biguint!(2u64);
            sc.price_observations().push(&new_observation.clone());
            let final_observation = sc.price_observations().get(1);
            assert_eq!(
                new_observation.lp_supply_accumulated,
                final_observation.lp_supply_accumulated
            );
        },
    );
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
    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1011,
        1019,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        100_099,
        MEX_TOKEN_ID,
        99_900,
    );

    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1020,
        1030,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        100_199,
        MEX_TOKEN_ID,
        99_801,
    );

    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1030,
        1040,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        100_249,
        MEX_TOKEN_ID,
        99_751,
    );

    // Simulate old price observations
    pair_setup.set_price_observation_as_old(1);
    pair_setup.set_price_observation_as_old(2);

    // Check migration safe price
    // Both observations are old
    // Latest LP amount is used, so this should be the different than before
    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1011,
        1019,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        50124,
        MEX_TOKEN_ID,
        50_025,
    );

    // First observation is old and the last observation is migrated
    // Latest LP amount is used, so this should be the different than before
    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1020,
        1030,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        50_174,
        MEX_TOKEN_ID,
        49_975,
    );

    // Both observations are migrated,
    // Saved LP is used, so this should be the same as before
    pair_setup.check_lp_tokens_safe_price(
        &pair_address,
        1030,
        1040,
        lp_token_amount,
        WEGLD_TOKEN_ID,
        100_249,
        MEX_TOKEN_ID,
        99_751,
    );
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

    pair_setup.check_price_observation(
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(1040);
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

    pair_setup.check_price_observation(
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
    pair_setup.set_block_round(block_round);
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
    pair_setup.set_block_round(block_round);
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
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);

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
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);
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
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);
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
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);

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

    block_round += weight;
    expected_amount -= 2; // slippage
    pair_setup.set_block_round(block_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // After second swap, weight = 1 + 10 = 11 >= 10, so finalization happens
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(
                observation.weight_accumulated,
                (starting_weight + weight) * 6_000
            );
        })
        .assert_ok();

    block_round += weight;
    expected_amount -= 2;
    pair_setup.set_block_round(block_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // After third swap, another save happens because the elapsed duration reaches the interval.
    // The new observation continues accumulating from the previous one
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 2);
            let second_observation = sc.price_observations().get(2);
            // Weight continues accumulating: 11 (from first) + 10 = 21
            assert_eq!(second_observation.weight_accumulated, 21u64 * 6_000);
        })
        .assert_ok();

    block_round += weight;
    expected_amount -= 2;
    pair_setup.set_block_round(block_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(block_round * 6_000);
    pair_setup.swap_fixed_input(
        WEGLD_TOKEN_ID,
        payment_amount,
        MEX_TOKEN_ID,
        900,
        expected_amount,
    );

    // After fourth swap, another direct save happens
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 3);
            let third_observation = sc.price_observations().get(3);
            // Weight continues accumulating: 21 (from second) + 10 = 31
            assert_eq!(third_observation.weight_accumulated, 31u64 * 6_000);
        })
        .assert_ok();
}

#[test]
fn test_safe_price_uses_millisecond_weights_for_save_interval() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);

    let starting_round = 10_000u64;
    let starting_timestamp_ms = 1_000_000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_timestamp_ms);

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.weight_accumulated, 6_000u64);
            assert_eq!(observation.recording_timestamp, starting_timestamp_ms);
        })
        .assert_ok();

    pair_setup.set_block_round(starting_round + 1);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_timestamp_ms + 600);

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_001_000),
                    &managed_biguint!(999_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
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
fn test_update_safe_price_converts_legacy_observation_before_save() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);
    pair_setup.set_block_round(91u64);

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
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

                sc.price_observations().push(&legacy_observation);
                sc.safe_price_current_index().set(1usize);

                sc.update_safe_price(
                    &managed_biguint!(100u64),
                    &managed_biguint!(200u64),
                    &managed_biguint!(1_000u64),
                );
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.safe_price_current_index().get(), 2usize);

            let observation = sc.price_observations().get(2);
            assert_eq!(observation.recording_round, 91u64);
            assert_eq!(observation.recording_timestamp, 91u64 * 6_000);
            assert_eq!(observation.weight_accumulated, 18_000u64);
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(660_000u64)
            );
            assert_eq!(
                observation.second_token_reserve_accumulated,
                managed_biguint!(1_320_000u64)
            );
            assert_eq!(
                observation.lp_supply_accumulated,
                managed_biguint!(6_000_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_timestamp_offset_ignores_duplicate_current_observation_after_save() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);

    let starting_round = 10_000u64;
    let starting_timestamp_ms = 1_000_000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_timestamp_ms);

    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    pair_setup.set_block_round(starting_round + 1);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_timestamp_ms + 6_000);

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

            let target_round =
                sc.get_round_by_timestamp_offset(6u64, managed_address!(&pair_address));
            let target_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), target_round);
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
fn test_safe_price_zero_timestamp_observation_uses_legacy_round_duration() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round(200u64);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(1_700_000_060_000u64);

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
                    lp_supply_accumulated: managed_biguint!(5u64),
                };
                let timestamped_reference_observation = PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(60_000u64),
                    second_token_reserve_accumulated: managed_biguint!(120_000u64),
                    weight_accumulated: 12_000u64,
                    recording_round: 100u64,
                    recording_timestamp: 1_700_000_000_000u64,
                    lp_supply_accumulated: managed_biguint!(30_000u64),
                };

                sc.price_observations().push(&legacy_observation);
                sc.price_observations()
                    .push(&timestamped_reference_observation);
                sc.safe_price_current_index().set(2usize);
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
            assert_eq!(
                normalized.lp_supply_accumulated,
                managed_biguint!(30_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_price_observation_after_latest_does_not_accumulate_without_new_timestamp() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.b_mock.set_block_round(101u64);
    pair_setup.b_mock.set_block_timestamp_ms(1_000_000u64);

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
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let observation =
                sc.get_price_observation_view(managed_address!(&pair_address), 101u64);

            assert_eq!(observation.recording_round, 101u64);
            assert_eq!(observation.recording_timestamp, 1_000_000u64);
            assert_eq!(observation.weight_accumulated, 6_000u64);
            assert_eq!(
                observation.first_token_reserve_accumulated,
                managed_biguint!(60_000u64)
            );
            assert_eq!(
                observation.second_token_reserve_accumulated,
                managed_biguint!(120_000u64)
            );
            assert_eq!(
                observation.lp_supply_accumulated,
                managed_biguint!(300_000u64)
            );
        })
        .assert_ok();
}

#[test]
fn test_safe_price_timestamp_offset_uses_current_price_observation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
    let pair_address = pair_setup.pair_wrapper.address_ref().clone();

    pair_setup.set_block_round(110u64);
    pair_setup.b_mock.set_block_timestamp_ms(606_000u64);

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
                    first_token_reserve_accumulated: managed_biguint!(606_000u64),
                    second_token_reserve_accumulated: managed_biguint!(1_212_000u64),
                    weight_accumulated: 606_000u64,
                    recording_round: 110u64,
                    recording_timestamp: 606_000u64,
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
            let target_round =
                sc.get_round_by_timestamp_offset(3u64, managed_address!(&pair_address));
            let target_observation =
                sc.get_price_observation_view(managed_address!(&pair_address), target_round);
            assert_eq!(target_observation.recording_round, 105u64);
            assert_eq!(target_observation.recording_timestamp, 603_000u64);
            assert_eq!(target_observation.weight_accumulated, 603_000u64);
            assert_eq!(
                target_observation.first_token_reserve_accumulated,
                managed_biguint!(603_000u64)
            );
            assert_eq!(
                target_observation.second_token_reserve_accumulated,
                managed_biguint!(1_206_000u64)
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
    pair_setup.b_mock.set_block_round(current_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(current_round * 6_000);

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

            let target_round =
                sc.get_round_by_timestamp_offset(6u64, managed_address!(&pair_address));
            assert_eq!(target_round, max_observations + 2);
        })
        .assert_ok();
}

// Test is commented as it needs a variable change in order to run succesfully
// In order to run the test with the current setup, MAX_OBSERVATIONS const must be set to 100
// This is necessary as using the MAINNET variable requires too many operations for a unit test
// #[test]
// fn test_safe_price_max_length() {
//     let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);
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
//         pair_setup.set_block_round(block_round);
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
//     pair_setup.set_block_round(block_round);
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
//         pair_setup.set_block_round(block_round);
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

//     let mut safe_price_round_offset = 20;
//     let mut safe_price_expected_amount = 30_894;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_round_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );

//     safe_price_round_offset = 10;
//     safe_price_expected_amount = 31_820;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_round_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );

//     safe_price_round_offset = 1;
//     safe_price_expected_amount = 32_038;
//     pair_setup.check_safe_price(
//         &pair_address,
//         block_round - safe_price_round_offset,
//         block_round,
//         WEGLD_TOKEN_ID,
//         first_token_payment_amount,
//         MEX_TOKEN_ID,
//         safe_price_expected_amount,
//     );
// }

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
fn test_intermediate_price_observation_accumulation() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // Set 5 legacy rounds, expressed as milliseconds.
    pair_setup.set_safe_price_timestamp_save_interval(5u64 * 6_000);

    let starting_round = 1000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_round * 6_000);

    // Add initial liquidity
    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // Manually trigger price update by calling update_safe_price directly
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                // Simulate price update with current reserves
                sc.update_safe_price(
                    &managed_biguint!(1_002_000), // first_token_reserve
                    &managed_biguint!(999_004),   // second_token_reserve
                    &managed_biguint!(1_001_000), // lp_supply
                );
            },
        )
        .assert_ok();

    // Check that main price observations storage is still empty
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
        })
        .assert_ok();

    // Check that intermediate observation exists with correct initial values
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.recording_round, starting_round);
            assert_eq!(intermediate.weight_accumulated, 6_000u64);
            // Should have accumulated values for round 1000
            assert!(intermediate.first_token_reserve_accumulated > managed_biguint!(0));
            assert!(intermediate.second_token_reserve_accumulated > managed_biguint!(0));
        })
        .assert_ok();

    // Second update at round 1002 - should update intermediate observation
    pair_setup.set_block_round(starting_round + 2);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 2) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_003_000),
                    &managed_biguint!(998_008),
                    &managed_biguint!(1_001_000),
                );
            },
        )
        .assert_ok();

    // Check intermediate observation accumulated more data (no finalization yet)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // Still no finalized observations
            assert_eq!(sc.price_observations().len(), 0);
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.recording_round, starting_round + 2); // Updated to round 1002
            assert_eq!(intermediate.weight_accumulated, 18_000u64); // 3 legacy rounds in ms
        })
        .assert_ok();

    // Third update at round 1005 - accumulated duration reaches the save interval
    pair_setup.set_block_round(starting_round + 5);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 5) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_004_000),
                    &managed_biguint!(997_012),
                    &managed_biguint!(1_001_000),
                );
            },
        )
        .assert_ok();

    // Verify finalization happened (weight = 6 >= 5)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // One finalized observation
            assert_eq!(sc.price_observations().len(), 1);
            // Check the finalized observation
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round + 5);
            assert_eq!(observation.weight_accumulated, 36_000u64); // 6 legacy rounds in ms
        })
        .assert_ok();
}

#[test]
fn test_intermediate_observation_finalization() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // Set 3 legacy rounds, expressed as milliseconds.
    pair_setup.set_safe_price_timestamp_save_interval(3u64 * 6_000);

    let starting_round = 2000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_round * 6_000);

    // Initialize safe price index
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                if sc.safe_price_current_index().is_empty() {
                    sc.safe_price_current_index().set(0);
                }
            },
        )
        .assert_ok();

    // First update - creates intermediate observation
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Second update at round 2001 - updates intermediate
    pair_setup.set_block_round(starting_round + 1);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 1) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_001_000),
                    &managed_biguint!(999_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Verify intermediate state - no finalization because no prior observation
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // No finalized observations yet
            assert_eq!(sc.price_observations().len(), 0);
            assert!(!sc.current_price_observation().is_empty());
            let intermediate = sc.current_price_observation().get();
            assert_eq!(intermediate.weight_accumulated, 12_000u64);
        })
        .assert_ok();

    // Third update at round 2003 - accumulated duration reaches the save interval
    pair_setup.set_block_round(starting_round + 3);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 3) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_002_000),
                    &managed_biguint!(998_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Verify finalization happened (weight = 4 >= 3)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // One finalized observation
            assert_eq!(sc.price_observations().len(), 1);

            // Check the finalized observation
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round + 3);
            assert_eq!(observation.weight_accumulated, 24_000u64);

            // Current index is now 1
            assert_eq!(sc.safe_price_current_index().get(), 1);
        })
        .assert_ok();
}

#[test]
fn test_one_legacy_round_timestamp_interval_saves_immediately() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // Set the interval to one legacy-round equivalent.
    pair_setup.set_safe_price_timestamp_save_interval(6_000u64);

    let starting_round = 3000u64;
    pair_setup.set_block_round(starting_round);

    // Add initial liquidity
    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // First price update should immediately save to main storage
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Check that observation was saved directly to main storage
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round);
            assert_eq!(observation.weight_accumulated, 6_000u64);

            assert_eq!(
                sc.current_price_observation().get().recording_round,
                observation.recording_round
            );
        })
        .assert_ok();

    // Second update at next round should also save immediately
    pair_setup.set_block_round(starting_round + 1);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_001_000),
                    &managed_biguint!(999_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Should have 2 observations now
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

    pair_setup.set_safe_price_timestamp_save_interval(3u64 * 6_000);

    let starting_round = 4000u64;
    pair_setup.set_block_round(starting_round);

    // Test with zero first token reserve - should return early
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(0), // zero first reserve
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Should not create any observations
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(sc.current_price_observation().is_empty());
        })
        .assert_ok();

    // Test with zero second token reserve
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(0), // zero second reserve
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Still should not create any observations
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(sc.current_price_observation().is_empty());
        })
        .assert_ok();

    // Test with zero LP supply
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(0), // zero LP supply
                );
            },
        )
        .assert_ok();

    // Still should not create any observations
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 0);
            assert!(sc.current_price_observation().is_empty());
        })
        .assert_ok();
}

#[test]
fn test_direct_save_when_interval_exceeded() {
    let mut pair_setup = PairSetup::new(pair::contract_obj, router::contract_obj);

    // Set the interval to five legacy-round equivalents.
    pair_setup.set_safe_price_timestamp_save_interval(5u64 * 6_000);

    let starting_round = 5000u64;
    pair_setup.set_block_round(starting_round);
    pair_setup
        .b_mock
        .set_block_timestamp_ms(starting_round * 6_000);

    // Add initial liquidity
    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // First update - creates intermediate observation
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Verify intermediate observation was created but no finalized observations yet
    // (weight = 1 < 5)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert!(!sc.current_price_observation().is_empty());
            assert_eq!(sc.price_observations().len(), 0);
        })
        .assert_ok();

    // Update at round 5005 - weight = 6 >= 5, finalization happens
    pair_setup.set_block_round(starting_round + 5);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 5) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_001_000),
                    &managed_biguint!(999_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Finalization happened (weight = 1 + 5 = 6 >= 5)
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            assert_eq!(sc.price_observations().len(), 1);
            let observation = sc.price_observations().get(1);
            assert_eq!(observation.recording_round, starting_round + 5);
        })
        .assert_ok();

    // Update after another six legacy-round equivalents, so another save happens.
    pair_setup.set_block_round(starting_round + 5 + 6);
    pair_setup
        .b_mock
        .set_block_timestamp_ms((starting_round + 5 + 6) * 6_000);
    pair_setup
        .b_mock
        .execute_tx(
            pair_setup.pair_wrapper.address_ref(),
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_safe_price(
                    &managed_biguint!(1_002_000),
                    &managed_biguint!(998_000),
                    &managed_biguint!(1_000_000),
                );
            },
        )
        .assert_ok();

    // Verify another save happened once the elapsed duration reached the interval.
    pair_setup
        .b_mock
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            // Two finalized observations
            assert_eq!(sc.price_observations().len(), 2);

            // Check second observation
            let second_obs = sc.price_observations().get(2);
            assert_eq!(second_obs.recording_round, starting_round + 5 + 6);

            // Weight should be accumulated: first observation plus the elapsed duration.
            assert_eq!(second_obs.weight_accumulated, 72_000u64);

            assert_eq!(
                sc.current_price_observation().get().recording_round,
                second_obs.recording_round
            );
        })
        .assert_ok();
}
