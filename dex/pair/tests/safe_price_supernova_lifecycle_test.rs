use multiversx_sc::{
    api::ManagedTypeApi,
    codec::{
        self,
        derive::{NestedDecode, NestedEncode, TopDecode, TopEncode},
    },
    imports::StorageMapper,
    storage::{mappers::VecMapper, StorageKey},
    types::{TestAddress, TestSCAddress, TimestampMillis},
};
use multiversx_sc_scenario::imports::*;
use pair::{
    config::ConfigModule,
    pair_actions::swap::SwapModule,
    pair_actions::{add_liq::AddLiquidityModule, remove_liq::RemoveLiquidityModule},
    safe_price::{PriceObservation, SafePriceModule, MAX_OBSERVATIONS},
    safe_price_view::SafePriceViewModule,
    Pair,
};
use pausable::{PausableModule, State};

// Safe Price views below are invoked through ScenarioWorld whitebox Rust modules. This covers
// view semantics and cross-account storage integration, not safe-price-view.wasm ABI, export,
// label, or VM dispatch behavior.
const OWNER: TestAddress = TestAddress::new("owner");
const USER: TestAddress = TestAddress::new("user");
const ROUTER: TestAddress = TestAddress::new("router");
const PAIR: TestSCAddress = TestSCAddress::new("pair");

const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

const INITIAL_FIRST_RESERVE: u64 = 1_008_000;
const INITIAL_SECOND_RESERVE: u64 = 2_016_000;
const LP_SUPPLY: u64 = 1_008_000;
const SWAP_AMOUNT: u64 = 1_008_000;
const QUOTE_INPUT: u64 = 420_001;
const LP_QUOTE: u64 = 420_001;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Debug)]
struct LegacyPriceObservation<M: ManagedTypeApi> {
    first_token_reserve_accumulated: BigUint<M>,
    second_token_reserve_accumulated: BigUint<M>,
    weight_accumulated: u64,
    recording_round: u64,
}

#[derive(Clone, Copy, Debug)]
struct ConstantReserveSegment {
    start_timestamp_ms: u64,
    end_timestamp_ms: u64,
    first_reserve: u64,
    second_reserve: u64,
    lp_supply: u64,
}

#[derive(Default)]
struct ReferenceLedger {
    segments: Vec<ConstantReserveSegment>,
}

#[derive(Debug)]
struct ExpectedWindow {
    weight_ms: u64,
    first_reserve_weighted: u128,
    second_reserve_weighted: u128,
    lp_supply_weighted: u128,
    first_to_second: u64,
    second_to_first: u64,
    first_to_second_remainder: u128,
    second_to_first_remainder: u128,
    lp_first: u64,
    lp_second: u64,
    lp_first_remainder: u128,
    lp_second_remainder: u128,
}

impl ReferenceLedger {
    fn push(
        &mut self,
        start_timestamp_ms: u64,
        end_timestamp_ms: u64,
        first_reserve: u64,
        second_reserve: u64,
    ) {
        self.push_with_lp_supply(
            start_timestamp_ms,
            end_timestamp_ms,
            first_reserve,
            second_reserve,
            LP_SUPPLY,
        );
    }

    fn push_with_lp_supply(
        &mut self,
        start_timestamp_ms: u64,
        end_timestamp_ms: u64,
        first_reserve: u64,
        second_reserve: u64,
        lp_supply: u64,
    ) {
        assert!(end_timestamp_ms > start_timestamp_ms);
        self.segments.push(ConstantReserveSegment {
            start_timestamp_ms,
            end_timestamp_ms,
            first_reserve,
            second_reserve,
            lp_supply,
        });
    }

    fn expected(&self, start_timestamp_ms: u64, end_timestamp_ms: u64) -> ExpectedWindow {
        assert!(end_timestamp_ms > start_timestamp_ms);

        let mut covered_ms = 0u64;
        let mut first_reserve_weighted = 0u128;
        let mut second_reserve_weighted = 0u128;
        let mut lp_supply_weighted = 0u128;

        for segment in &self.segments {
            let overlap_start = core::cmp::max(start_timestamp_ms, segment.start_timestamp_ms);
            let overlap_end = core::cmp::min(end_timestamp_ms, segment.end_timestamp_ms);
            if overlap_end <= overlap_start {
                continue;
            }

            let weight_ms = overlap_end - overlap_start;
            covered_ms += weight_ms;
            first_reserve_weighted += u128::from(weight_ms) * u128::from(segment.first_reserve);
            second_reserve_weighted += u128::from(weight_ms) * u128::from(segment.second_reserve);
            lp_supply_weighted += u128::from(weight_ms) * u128::from(segment.lp_supply);
        }

        let weight_ms = end_timestamp_ms - start_timestamp_ms;
        assert_eq!(
            covered_ms, weight_ms,
            "reference ledger does not cover the complete requested window"
        );

        let average_first = first_reserve_weighted / u128::from(weight_ms);
        let average_second = second_reserve_weighted / u128::from(weight_ms);
        let average_lp_supply = lp_supply_weighted / u128::from(weight_ms);

        let first_to_second_numerator = u128::from(QUOTE_INPUT) * average_second;
        let second_to_first_numerator = u128::from(QUOTE_INPUT) * average_first;
        let lp_first_numerator = u128::from(LP_QUOTE) * average_first;
        let lp_second_numerator = u128::from(LP_QUOTE) * average_second;

        ExpectedWindow {
            weight_ms,
            first_reserve_weighted,
            second_reserve_weighted,
            lp_supply_weighted,
            first_to_second: u64::try_from(first_to_second_numerator / average_first).unwrap(),
            second_to_first: u64::try_from(second_to_first_numerator / average_second).unwrap(),
            first_to_second_remainder: first_to_second_numerator % average_first,
            second_to_first_remainder: second_to_first_numerator % average_second,
            lp_first: u64::try_from(lp_first_numerator / average_lp_supply).unwrap(),
            lp_second: u64::try_from(lp_second_numerator / average_lp_supply).unwrap(),
            lp_first_remainder: lp_first_numerator % average_lp_supply,
            lp_second_remainder: lp_second_numerator % average_lp_supply,
        }
    }
}

fn setup_pair(round_time_ms: u64, block_round: u64, block_timestamp_ms: u64) -> ScenarioWorld {
    let mut world = ScenarioWorld::debugger();
    world.register_contract("0x0500", pair::ContractBuilder);
    world.account(OWNER).balance(0u64);
    world
        .account(ROUTER)
        .balance(0u64)
        .storage_mandos("str:default_safe_price_timestamp_offset", "u64:6000")
        .storage_mandos("str:safe_price_timestamp_save_interval", "u64:6000");
    world
        .account(USER)
        .balance(0u64)
        .esdt_balance(TestTokenIdentifier::new("WEGLD-abcdef"), 100_000_000u64)
        .esdt_balance(TestTokenIdentifier::new("MEX-abcdef"), 100_000_000u64);
    world.block_round_time_ms(round_time_ms);
    set_block(&mut world, block_round, block_timestamp_ms);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(BytesValue::from_hex("0500"))
        .new_address(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
            let second_token_id = managed_token_id!(MEX_TOKEN_ID);
            sc.init(
                first_token_id.clone(),
                second_token_id.clone(),
                ROUTER.to_managed_address(),
                OWNER.to_managed_address(),
                0u64,
                0u64,
                ManagedAddress::zero(),
                MultiValueEncoded::new(),
            );
            sc.lp_token_identifier().set(managed_token_id!(LP_TOKEN_ID));
            sc.pair_reserve(&first_token_id)
                .set(managed_biguint!(INITIAL_FIRST_RESERVE));
            sc.pair_reserve(&second_token_id)
                .set(managed_biguint!(INITIAL_SECOND_RESERVE));
            sc.lp_token_supply().set(managed_biguint!(LP_SUPPLY));
            sc.state().set(State::Active);
            assert_eq!(sc.router_address().get(), ROUTER.to_managed_address());
        });
    world.set_esdt_local_roles(
        PAIR,
        LP_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );

    for (token_id, amount) in [
        ("WEGLD-abcdef", INITIAL_FIRST_RESERVE),
        ("MEX-abcdef", INITIAL_SECOND_RESERVE),
    ] {
        world
            .tx()
            .from(USER)
            .to(PAIR)
            .payment(Payment::<StaticApi>::try_new(token_id, 0u64, amount).unwrap())
            .whitebox(pair::contract_obj, |_sc| {});
    }

    world
}

fn set_block(world: &mut ScenarioWorld, block_round: u64, block_timestamp_ms: u64) {
    world
        .current_block()
        .block_round(block_round)
        .block_timestamp_millis(TimestampMillis::new(block_timestamp_ms));
}

fn swap_first_for_second(world: &mut ScenarioWorld) {
    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("WEGLD-abcdef", 0u64, SWAP_AMOUNT).unwrap())
        .whitebox(pair::contract_obj, |sc| {
            let output =
                sc.swap_tokens_fixed_input(managed_token_id!(MEX_TOKEN_ID), managed_biguint!(1u64));
            assert_eq!(output.token_identifier, managed_token_id!(MEX_TOKEN_ID));
            assert_eq!(output.amount, managed_biguint!(SWAP_AMOUNT));
        });
}

fn swap_second_for_first(world: &mut ScenarioWorld) {
    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("MEX-abcdef", 0u64, SWAP_AMOUNT).unwrap())
        .whitebox(pair::contract_obj, |sc| {
            let output = sc
                .swap_tokens_fixed_input(managed_token_id!(WEGLD_TOKEN_ID), managed_biguint!(1u64));
            assert_eq!(output.token_identifier, managed_token_id!(WEGLD_TOKEN_ID));
            assert_eq!(output.amount, managed_biguint!(SWAP_AMOUNT));
        });
}

fn swap_first_for_fixed_second_output(world: &mut ScenarioWorld, output_amount: u64) {
    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("WEGLD-abcdef", 0u64, SWAP_AMOUNT).unwrap())
        .whitebox(pair::contract_obj, |sc| {
            let (output, residuum) = sc
                .swap_tokens_fixed_output(
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_biguint!(output_amount),
                )
                .into_tuple();
            assert_eq!(output.token_identifier, managed_token_id!(MEX_TOKEN_ID));
            assert_eq!(output.amount, managed_biguint!(output_amount));
            assert_eq!(residuum.token_identifier, managed_token_id!(WEGLD_TOKEN_ID));
            assert!(residuum.amount > 0u64);
        });
}

fn add_proportional_liquidity(world: &mut ScenarioWorld, first_amount: u64, second_amount: u64) {
    let mut payments = PaymentVec::<StaticApi>::new();
    payments.push(Payment::try_new("WEGLD-abcdef", 0u64, first_amount).unwrap());
    payments.push(Payment::try_new("MEX-abcdef", 0u64, second_amount).unwrap());

    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(payments)
        .whitebox(pair::contract_obj, |sc| {
            let (lp_payment, first_added, second_added) = sc
                .add_liquidity(managed_biguint!(1u64), managed_biguint!(1u64))
                .into_tuple();
            assert_eq!(lp_payment.token_identifier, managed_token_id!(LP_TOKEN_ID));
            assert_eq!(lp_payment.amount, managed_biguint!(first_amount));
            assert_eq!(first_added.amount, managed_biguint!(first_amount));
            assert_eq!(second_added.amount, managed_biguint!(second_amount));
        });
}

fn remove_liquidity_position(world: &mut ScenarioWorld, lp_amount: u64) {
    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("LPTOK-abcdef", 0u64, lp_amount).unwrap())
        .whitebox(pair::contract_obj, |sc| {
            let (first_payment, second_payment) = sc
                .remove_liquidity(managed_biguint!(1u64), managed_biguint!(1u64))
                .into_tuple();
            assert_eq!(
                first_payment.token_identifier,
                managed_token_id!(WEGLD_TOKEN_ID)
            );
            assert_eq!(
                second_payment.token_identifier,
                managed_token_id!(MEX_TOKEN_ID)
            );
            assert!(first_payment.amount > 0u64);
            assert!(second_payment.amount > 0u64);
        });
}

fn set_raw_router_default_offset(world: &mut ScenarioWorld, offset_ms: u64) {
    set_raw_router_config(world, offset_ms, 6_000u64);
}

fn set_raw_router_config(world: &mut ScenarioWorld, default_offset_ms: u64, save_interval_ms: u64) {
    let encoded_default_offset = match default_offset_ms {
        600u64 => "u64:600",
        3_000u64 => "u64:3000",
        6_000u64 => "u64:6000",
        12_000u64 => "u64:12000",
        18_000u64 => "u64:18000",
        24_000u64 => "u64:24000",
        42_000u64 => "u64:42000",
        60_000u64 => "u64:60000",
        _ => panic!("unsupported deterministic default offset: {default_offset_ms}"),
    };
    let encoded_save_interval = match save_interval_ms {
        6_000u64 => "u64:6000",
        6_500u64 => "u64:6500",
        _ => panic!("unsupported deterministic save interval: {save_interval_ms}"),
    };
    let mut router = Account::new().balance(0u64);
    router.storage.insert(
        "str:default_safe_price_timestamp_offset".into(),
        encoded_default_offset.into(),
    );
    router.storage.insert(
        "str:safe_price_timestamp_save_interval".into(),
        encoded_save_interval.into(),
    );
    world.set_state_step(SetStateStep::new().put_account(ROUTER, router));
}

fn remove_raw_router_save_interval(world: &mut ScenarioWorld) {
    let mut router = Account::new().balance(0u64);
    router.storage.insert(
        "str:default_safe_price_timestamp_offset".into(),
        "u64:6000".into(),
    );
    world.set_state_step(SetStateStep::new().put_account(ROUTER, router));
}

fn assert_failed_action_preserved_upgraded_baseline(world: &mut ScenarioWorld) {
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.price_observations().len(), 3usize);
        assert_eq!(sc.safe_price_current_index().get(), 3usize);
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_round, 102u64);
        assert_eq!(current.recording_timestamp, 612_000u64);
        assert_eq!(current.weight_accumulated, 612_000u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64)
        );
        assert_eq!(current.lp_supply_accumulated, managed_biguint!(0u64));
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get(),
            managed_biguint!(INITIAL_FIRST_RESERVE)
        );
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get(),
            managed_biguint!(INITIAL_SECOND_RESERVE)
        );
    });
    world
        .check_account(USER)
        .esdt_balance(
            TestTokenIdentifier::new("WEGLD-abcdef"),
            100_000_000u64 - INITIAL_FIRST_RESERVE,
        )
        .esdt_balance(
            TestTokenIdentifier::new("MEX-abcdef"),
            100_000_000u64 - INITIAL_SECOND_RESERVE,
        );
    world
        .check_account(PAIR)
        .esdt_balance(
            TestTokenIdentifier::new("WEGLD-abcdef"),
            INITIAL_FIRST_RESERVE,
        )
        .esdt_balance(
            TestTokenIdentifier::new("MEX-abcdef"),
            INITIAL_SECOND_RESERVE,
        );
}

fn assert_token_quote_by_timestamp_range(
    world: &mut ScenarioWorld,
    start_timestamp_ms: u64,
    end_timestamp_ms: u64,
    expected_amount: u64,
) {
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let quote = sc.get_safe_price_by_timestamp_range(
            PAIR.to_managed_address(),
            start_timestamp_ms,
            end_timestamp_ms,
            EsdtTokenPayment::new(
                managed_token_id!(WEGLD_TOKEN_ID),
                0,
                managed_biguint!(QUOTE_INPUT),
            ),
        );
        assert_eq!(quote.token_identifier, managed_token_id!(MEX_TOKEN_ID));
        assert_eq!(quote.amount, managed_biguint!(expected_amount));
    });
}

fn assert_finalized_cumulative_delta(
    world: &mut ScenarioWorld,
    start_index: usize,
    end_index: usize,
    duration_ms: u64,
    first_reserve: u64,
    second_reserve: u64,
    lp_supply: u64,
) {
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let start = sc.price_observations().get(start_index);
        let end = sc.price_observations().get(end_index);
        assert_eq!(
            &end.first_token_reserve_accumulated - &start.first_token_reserve_accumulated,
            managed_biguint!(duration_ms * first_reserve)
        );
        assert_eq!(
            &end.second_token_reserve_accumulated - &start.second_token_reserve_accumulated,
            managed_biguint!(duration_ms * second_reserve)
        );
        assert_eq!(
            &end.lp_supply_accumulated - &start.lp_supply_accumulated,
            managed_biguint!(duration_ms * lp_supply)
        );
        assert_eq!(
            end.weight_accumulated - start.weight_accumulated,
            duration_ms
        );
        assert_eq!(
            end.recording_timestamp - start.recording_timestamp,
            duration_ms
        );
    });
}

fn assert_default_offset_uses_expected_window(
    world: &mut ScenarioWorld,
    ledger: &ReferenceLedger,
    configured_offset_ms: u64,
    expected_start_timestamp_ms: u64,
    end_timestamp_ms: u64,
) {
    set_raw_router_config(world, configured_offset_ms, 6_000u64);
    let expected = ledger.expected(expected_start_timestamp_ms, end_timestamp_ms);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let token_quote = sc.get_safe_price_by_default_offset(
            PAIR.to_managed_address(),
            EsdtTokenPayment::new(
                managed_token_id!(WEGLD_TOKEN_ID),
                0,
                managed_biguint!(QUOTE_INPUT),
            ),
        );
        assert_eq!(
            token_quote.token_identifier,
            managed_token_id!(MEX_TOKEN_ID)
        );
        assert_eq!(
            token_quote.amount,
            managed_biguint!(expected.first_to_second)
        );

        let (first_lp_quote, second_lp_quote) = sc
            .get_lp_tokens_safe_price_by_default_offset(
                PAIR.to_managed_address(),
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple();
        assert_eq!(first_lp_quote.amount, managed_biguint!(expected.lp_first));
        assert_eq!(second_lp_quote.amount, managed_biguint!(expected.lp_second));
    });
}

fn seed_four_field_legacy_history_and_upgrade(world: &mut ScenarioWorld) -> ReferenceLedger {
    // Unit tests cannot execute the old deployed WASM. This is the only direct storage fixture:
    // it writes the deployed four-field value shape, then all lifecycle actions use current
    // production entrypoints and the real Pair upgrade handler.
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let mut legacy_observations =
                VecMapper::<DebugApi, LegacyPriceObservation<DebugApi>>::new(StorageKey::new(
                    b"price_observations",
                ));
            for observation in [
                LegacyPriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(100_800_000u64),
                    second_token_reserve_accumulated: managed_biguint!(201_600_000u64),
                    weight_accumulated: 100u64,
                    recording_round: 100u64,
                },
                LegacyPriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(101_808_000u64),
                    second_token_reserve_accumulated: managed_biguint!(203_616_000u64),
                    weight_accumulated: 101u64,
                    recording_round: 101u64,
                },
                LegacyPriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(102_816_000u64),
                    second_token_reserve_accumulated: managed_biguint!(205_632_000u64),
                    weight_accumulated: 102u64,
                    recording_round: 102u64,
                },
            ] {
                legacy_observations.push(&observation);
            }
            sc.safe_price_current_index().set(3usize);

            let decoded: PriceObservation<DebugApi> = sc.price_observations().get(3usize);
            assert_eq!(decoded.recording_round, 102u64);
            assert_eq!(decoded.recording_timestamp, 0u64);
            assert_eq!(decoded.lp_supply_accumulated, 0u64);
            assert!(sc.current_price_observation().is_empty());
            assert!(sc.safe_price_legacy_cutover().is_empty());
        });

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            assert_eq!(
                sc.blockchain()
                    .get_block_round_time_millis()
                    .as_u64_millis(),
                6_000u64
            );
            sc.upgrade();
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.safe_price_legacy_cutover().get(), (102u64, 612_000u64));
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_round, 102u64);
        assert_eq!(current.recording_timestamp, 612_000u64);
        assert_eq!(current.weight_accumulated, 612_000u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64)
        );
        assert_eq!(current.lp_supply_accumulated, 0u64);

        let still_four_field: PriceObservation<DebugApi> = sc.price_observations().get(1usize);
        assert_eq!(still_four_field.recording_timestamp, 0u64);
        assert_eq!(still_four_field.lp_supply_accumulated, 0u64);
    });

    let mut ledger = ReferenceLedger::default();
    ledger.push(
        600_000u64,
        606_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );
    ledger.push(
        606_000u64,
        612_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );
    ledger
}

fn upgrade_empty_legacy_history(world: &mut ScenarioWorld) {
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            assert_eq!(
                sc.blockchain()
                    .get_block_round_time_millis()
                    .as_u64_millis(),
                6_000u64
            );
            assert!(sc.price_observations().is_empty());
            sc.upgrade();
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert!(sc.price_observations().is_empty());
        assert!(sc.current_price_observation().is_empty());
        assert!(sc.safe_price_legacy_cutover().is_empty());
    });
}

fn assert_window_through_every_endpoint_family(
    world: &mut ScenarioWorld,
    ledger: &ReferenceLedger,
    start_round: u64,
    end_round: u64,
    start_timestamp_ms: u64,
    end_timestamp_ms: u64,
) {
    let expected = ledger.expected(start_timestamp_ms, end_timestamp_ms);
    let offset_ms = end_timestamp_ms - start_timestamp_ms;
    assert_eq!(expected.weight_ms, offset_ms);
    assert_eq!(
        offset_ms % 1_000,
        0,
        "legacy seconds endpoint needs an exact second window"
    );
    assert!(expected.first_reserve_weighted > 0);
    assert!(expected.second_reserve_weighted > 0);
    assert!(expected.lp_supply_weighted > 0);

    set_raw_router_default_offset(world, offset_ms);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.blockchain().get_block_round(), end_round);
        assert_eq!(
            sc.blockchain().get_block_timestamp_millis().as_u64_millis(),
            end_timestamp_ms
        );
        let pair_address = PAIR.to_managed_address();
        let round_offset = end_round - start_round;
        let offset_seconds = offset_ms / 1_000;
        let first_input = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(QUOTE_INPUT),
        );
        let second_input = EsdtTokenPayment::new(
            managed_token_id!(MEX_TOKEN_ID),
            0,
            managed_biguint!(QUOTE_INPUT),
        );

        let first_to_second_quotes = [
            sc.get_safe_price(
                pair_address.clone(),
                start_round,
                end_round,
                first_input.clone(),
            ),
            sc.get_safe_price_by_round_offset(
                pair_address.clone(),
                round_offset,
                first_input.clone(),
            ),
            sc.get_safe_price_by_timestamp_offset(
                pair_address.clone(),
                offset_seconds,
                first_input.clone(),
            ),
            sc.get_safe_price_by_timestamp_offset_ms(
                pair_address.clone(),
                offset_ms,
                first_input.clone(),
            ),
            sc.get_safe_price_by_default_offset(pair_address.clone(), first_input.clone()),
            sc.update_and_get_safe_price(first_input),
        ];
        for quote in first_to_second_quotes {
            assert_eq!(quote.token_identifier, managed_token_id!(MEX_TOKEN_ID));
            assert_eq!(quote.amount, managed_biguint!(expected.first_to_second));
        }

        let second_to_first_quotes = [
            sc.get_safe_price(
                pair_address.clone(),
                start_round,
                end_round,
                second_input.clone(),
            ),
            sc.get_safe_price_by_round_offset(
                pair_address.clone(),
                round_offset,
                second_input.clone(),
            ),
            sc.get_safe_price_by_timestamp_offset(
                pair_address.clone(),
                offset_seconds,
                second_input.clone(),
            ),
            sc.get_safe_price_by_timestamp_offset_ms(
                pair_address.clone(),
                offset_ms,
                second_input.clone(),
            ),
            sc.get_safe_price_by_default_offset(pair_address.clone(), second_input.clone()),
            sc.update_and_get_safe_price(second_input),
        ];
        for quote in second_to_first_quotes {
            assert_eq!(quote.token_identifier, managed_token_id!(WEGLD_TOKEN_ID));
            assert_eq!(quote.amount, managed_biguint!(expected.second_to_first));
        }

        let lp_quotes = [
            sc.get_lp_tokens_safe_price(
                pair_address.clone(),
                start_round,
                end_round,
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple(),
            sc.get_lp_tokens_safe_price_by_round_offset(
                pair_address.clone(),
                round_offset,
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple(),
            sc.get_lp_tokens_safe_price_by_timestamp_offset(
                pair_address.clone(),
                offset_seconds,
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple(),
            sc.get_lp_tokens_safe_price_by_timestamp_offset_ms(
                pair_address.clone(),
                offset_ms,
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple(),
            sc.get_lp_tokens_safe_price_by_default_offset(pair_address, managed_biguint!(LP_QUOTE))
                .into_tuple(),
            sc.update_and_get_tokens_for_given_position_with_safe_price(managed_biguint!(LP_QUOTE))
                .into_tuple(),
        ];
        for (first_payment, second_payment) in lp_quotes {
            assert_eq!(
                first_payment.token_identifier,
                managed_token_id!(WEGLD_TOKEN_ID)
            );
            assert_eq!(first_payment.amount, managed_biguint!(expected.lp_first));
            assert_eq!(
                second_payment.token_identifier,
                managed_token_id!(MEX_TOKEN_ID)
            );
            assert_eq!(second_payment.amount, managed_biguint!(expected.lp_second));
        }
    });
}

#[test]
fn fresh_post_supernova_pair_tracks_actual_swaps_at_600ms_cadence() {
    let mut world = setup_pair(600u64, 190u64, 114_000u64);
    let mut ledger = ReferenceLedger::default();

    // Bootstrap native history, then create the first finalized boundary after 6 seconds.
    swap_first_for_second(&mut world);

    set_block(&mut world, 200u64, 120_000u64);
    swap_second_for_first(&mut world);

    set_block(&mut world, 215u64, 129_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        120_000u64,
        129_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 220u64, 132_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        129_000u64,
        132_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    let expected = ledger.expected(120_000u64, 132_000u64);
    assert_ne!(expected.first_to_second, expected.second_to_first);
    assert!(expected.first_to_second_remainder > 0u128);
    assert!(expected.second_to_first_remainder > 0u128);
    assert!(expected.lp_first_remainder > 0u128);
    assert!(expected.lp_second_remainder > 0u128);

    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 200u64, 220u64, 120_000u64, 132_000u64,
    );
}

#[test]
fn legacy_pair_upgrade_before_supernova_preserves_every_reachable_window() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    let mut ledger = seed_four_field_legacy_history_and_upgrade(&mut world);

    // A-A: pre-upgrade/pre-Supernova legacy history, observed after the real upgrade.
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 100u64, 102u64, 600_000u64, 612_000u64,
    );

    // Phase B uses timestamp-millisecond accounting while runtime rounds are still 6 seconds.
    set_block(&mut world, 103u64, 618_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        612_000u64,
        618_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 104u64, 624_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        618_000u64,
        624_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    // A-B and B-B are both trailing windows at the final Phase-B block.
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 100u64, 104u64, 600_000u64, 624_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 103u64, 104u64, 618_000u64, 624_000u64,
    );

    // Phase C switches only the ScenarioWorld runtime cadence. The first action is pending
    // at +3s; the second reaches the configured 6s save interval and finalizes the aggregate.
    world.block_round_time_ms(600u64);
    set_block(&mut world, 109u64, 627_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        624_000u64,
        627_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 114u64, 630_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        627_000u64,
        630_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    // Create a finalized C boundary, then end on a pending current observation. This makes
    // a C-C window exactly reachable without assuming an unsaved breakpoint is queryable.
    set_block(&mut world, 129u64, 639_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        630_000u64,
        639_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 134u64, 642_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        639_000u64,
        642_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let finalized = sc
            .price_observations()
            .get(sc.safe_price_current_index().get());
        assert_eq!(finalized.recording_timestamp, 639_000u64);
        assert_eq!(
            sc.current_price_observation().get().recording_timestamp,
            642_000u64
        );
    });

    // A-C, B-C, and C-C complete the six reachable chronological window classes.
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 100u64, 134u64, 600_000u64, 642_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 103u64, 134u64, 618_000u64, 642_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 129u64, 134u64, 639_000u64, 642_000u64,
    );
}

#[test]
fn empty_legacy_pair_upgrade_then_post_supernova_swaps_build_native_history() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    upgrade_empty_legacy_history(&mut world);

    // No Phase-B swap occurs. The first two Phase-C swaps bootstrap the native history and
    // create its first finalized boundary under the unchanged 6,000ms save interval.
    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_second(&mut world);

    set_block(&mut world, 112u64, 618_000u64);
    swap_second_for_first(&mut world);

    let mut ledger = ReferenceLedger::default();
    set_block(&mut world, 127u64, 627_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        618_000u64,
        627_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 132u64, 630_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        627_000u64,
        630_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert!(sc.safe_price_legacy_cutover().is_empty());
        let oldest: PriceObservation<DebugApi> = sc.price_observations().get(1usize);
        assert!(oldest.recording_timestamp > 0);
        assert!(oldest.lp_supply_accumulated > 0u64);
    });

    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 112u64, 132u64, 618_000u64, 630_000u64,
    );
}

#[test]
fn populated_legacy_upgrade_without_phase_b_actions_preserves_a_c_and_c_c_windows() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    let mut ledger = seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 112u64, 618_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        612_000u64,
        618_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );

    set_block(&mut world, 122u64, 624_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        618_000u64,
        624_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 100u64, 122u64, 600_000u64, 624_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 112u64, 122u64, 618_000u64, 624_000u64,
    );
}

#[test]
fn first_post_supernova_action_exactly_six_hundred_ms_after_cutover_adds_one_round_weight() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_second(&mut world);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_round, 103u64);
        assert_eq!(current.recording_timestamp, 612_600u64);
        assert_eq!(current.weight_accumulated, 612_600u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64 + 600u64 * INITIAL_FIRST_RESERVE)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64 + 600u64 * INITIAL_SECOND_RESERVE)
        );
        assert_eq!(
            current.lp_supply_accumulated,
            managed_biguint!(600u64 * LP_SUPPLY)
        );
    });
}

#[test]
fn action_at_exact_cutover_timestamp_is_weight_noop_and_next_round_uses_post_action_reserves() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    swap_first_for_second(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.price_observations().len(), 3usize);
        assert_eq!(sc.safe_price_current_index().get(), 3usize);
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_timestamp, 612_000u64);
        assert_eq!(current.weight_accumulated, 612_000u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64)
        );
    });

    set_block(&mut world, 103u64, 612_600u64);
    swap_second_for_first(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_timestamp, 612_600u64);
        assert_eq!(current.weight_accumulated, 612_600u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64 + 600u64 * INITIAL_SECOND_RESERVE)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64 + 600u64 * INITIAL_FIRST_RESERVE)
        );
    });
}

#[test]
fn minimal_post_supernova_window_of_six_hundred_ms_is_queryable_by_ms_and_round() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_second(&mut world);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let input = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(QUOTE_INPUT),
        );
        let expected = managed_biguint!(QUOTE_INPUT * 2u64);
        let by_ms = sc.get_safe_price_by_timestamp_offset_ms(
            PAIR.to_managed_address(),
            600u64,
            input.clone(),
        );
        let by_round = sc.get_safe_price(PAIR.to_managed_address(), 102u64, 103u64, input);
        assert_eq!(by_ms.amount, expected);
        assert_eq!(by_round.amount, expected);
    });
}

#[test]
fn non_aligned_six_thousand_five_hundred_ms_save_interval_keeps_pending_then_finalizes() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);
    set_raw_router_config(&mut world, 6_000u64, 6_500u64);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 112u64, 618_000u64);
    swap_first_for_second(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.price_observations().len(), 3usize);
        assert_eq!(sc.safe_price_current_index().get(), 3usize);
        assert_eq!(
            sc.current_price_observation().get().recording_timestamp,
            618_000u64
        );
    });

    set_block(&mut world, 113u64, 618_600u64);
    swap_second_for_first(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.safe_price_current_index().get(), 4usize);
        assert_eq!(
            sc.price_observations().get(4usize).recording_timestamp,
            618_600u64
        );
        assert_eq!(
            sc.current_price_observation().get().recording_timestamp,
            618_600u64
        );
    });
}

#[test]
fn fresh_pair_writer_uses_future_two_hundred_ms_runtime_cadence() {
    let mut world = setup_pair(200u64, 100u64, 20_000u64);
    swap_first_for_second(&mut world);

    set_block(&mut world, 101u64, 20_200u64);
    swap_second_for_first(&mut world);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert!(sc.price_observations().is_empty());
        let current = sc.current_price_observation().get();
        assert_eq!(current.recording_round, 101u64);
        assert_eq!(current.recording_timestamp, 20_200u64);
        assert_eq!(current.weight_accumulated, 400u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(200u64 * INITIAL_FIRST_RESERVE + 200u64 * INITIAL_SECOND_RESERVE)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(200u64 * INITIAL_SECOND_RESERVE + 200u64 * INITIAL_FIRST_RESERVE)
        );
        assert_eq!(
            current.lp_supply_accumulated,
            managed_biguint!(400u64 * LP_SUPPLY)
        );
    });
}

#[test]
fn fixed_output_swap_post_supernova_weights_the_resulting_reserves_in_the_next_segment() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_fixed_second_output(&mut world, 504_000u64);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get(),
            managed_biguint!(1_344_001u64)
        );
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get(),
            managed_biguint!(1_512_000u64)
        );
    });

    set_block(&mut world, 113u64, 618_600u64);
    swap_first_for_fixed_second_output(&mut world, 252_000u64);

    let average_first =
        (u128::from(INITIAL_FIRST_RESERVE) * 600u128 + 1_344_001u128 * 6_000u128) / 6_600u128;
    let average_second =
        (u128::from(INITIAL_SECOND_RESERVE) * 600u128 + 1_512_000u128 * 6_000u128) / 6_600u128;
    let expected = u64::try_from(u128::from(QUOTE_INPUT) * average_second / average_first).unwrap();
    assert_token_quote_by_timestamp_range(&mut world, 612_000u64, 618_600u64, expected);
}

#[test]
fn add_liquidity_across_migration_weights_new_reserves_and_lp_supply() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    set_block(&mut world, 103u64, 618_000u64);
    add_proportional_liquidity(&mut world, 504_000u64, 1_008_000u64);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.lp_token_supply().get(), managed_biguint!(1_512_000u64));
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get(),
            managed_biguint!(1_512_000u64)
        );
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get(),
            managed_biguint!(3_024_000u64)
        );
    });

    world.block_round_time_ms(600u64);
    set_block(&mut world, 113u64, 624_000u64);
    swap_first_for_fixed_second_output(&mut world, 504_000u64);
    assert_finalized_cumulative_delta(
        &mut world,
        4usize,
        5usize,
        6_000u64,
        1_512_000u64,
        3_024_000u64,
        1_512_000u64,
    );

    let mut ledger = ReferenceLedger::default();
    ledger.push_with_lp_supply(
        618_000u64,
        624_000u64,
        1_512_000u64,
        3_024_000u64,
        1_512_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 103u64, 113u64, 618_000u64, 624_000u64,
    );
}

#[test]
fn remove_liquidity_post_supernova_weights_reduced_reserves_and_lp_supply() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    add_proportional_liquidity(&mut world, 504_000u64, 1_008_000u64);
    world.block_round_time_ms(600u64);
    set_block(&mut world, 112u64, 618_000u64);
    remove_liquidity_position(&mut world, 252_000u64);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.lp_token_supply().get(), managed_biguint!(1_260_000u64));
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get(),
            managed_biguint!(1_260_000u64)
        );
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get(),
            managed_biguint!(2_520_000u64)
        );
    });

    set_block(&mut world, 122u64, 624_000u64);
    swap_first_for_fixed_second_output(&mut world, 420_000u64);
    assert_finalized_cumulative_delta(
        &mut world,
        4usize,
        5usize,
        6_000u64,
        1_260_000u64,
        2_520_000u64,
        1_260_000u64,
    );

    let mut ledger = ReferenceLedger::default();
    ledger.push_with_lp_supply(
        618_000u64,
        624_000u64,
        1_260_000u64,
        2_520_000u64,
        1_260_000u64,
    );
    assert_window_through_every_endpoint_family(
        &mut world, &ledger, 112u64, 122u64, 618_000u64, 624_000u64,
    );
}

#[test]
fn two_real_actions_at_same_timestamp_add_weight_once_and_next_segment_uses_final_reserves() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_second(&mut world);
    swap_second_for_first(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let current = sc.current_price_observation().get();
        assert_eq!(current.weight_accumulated, 612_600u64);
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get(),
            managed_biguint!(INITIAL_FIRST_RESERVE)
        );
        assert_eq!(
            sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get(),
            managed_biguint!(INITIAL_SECOND_RESERVE)
        );
    });

    set_block(&mut world, 104u64, 613_200u64);
    swap_first_for_second(&mut world);
    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let current = sc.current_price_observation().get();
        assert_eq!(current.weight_accumulated, 613_200u64);
        assert_eq!(
            current.first_token_reserve_accumulated,
            managed_biguint!(616_896_000_000u64 + 1_200u64 * INITIAL_FIRST_RESERVE)
        );
        assert_eq!(
            current.second_token_reserve_accumulated,
            managed_biguint!(1_233_792_000_000u64 + 1_200u64 * INITIAL_SECOND_RESERVE)
        );
    });
}

#[test]
fn failed_swap_after_safe_price_update_rolls_back_oracle_reserves_and_balances() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);
    world.block_round_time_ms(600u64);
    set_block(&mut world, 112u64, 618_000u64);

    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("WEGLD-abcdef", 0u64, SWAP_AMOUNT).unwrap())
        .returns(ExpectError(4, "Slippage exceeded"))
        .whitebox(pair::contract_obj, |sc| {
            sc.swap_tokens_fixed_input(
                managed_token_id!(MEX_TOKEN_ID),
                managed_biguint!(1_500_000u64),
            );
        });

    assert_failed_action_preserved_upgraded_baseline(&mut world);
}

#[test]
fn missing_router_save_interval_reverts_real_swap_and_all_safe_price_side_effects() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);
    remove_raw_router_save_interval(&mut world);
    world.block_round_time_ms(600u64);
    set_block(&mut world, 112u64, 618_000u64);

    world
        .tx()
        .from(USER)
        .to(PAIR)
        .payment(Payment::<StaticApi>::try_new("WEGLD-abcdef", 0u64, SWAP_AMOUNT).unwrap())
        .returns(ExpectError(4, "Safe price timestamp save interval not set"))
        .whitebox(pair::contract_obj, |sc| {
            sc.swap_tokens_fixed_input(managed_token_id!(MEX_TOKEN_ID), managed_biguint!(1u64));
        });

    assert_failed_action_preserved_upgraded_baseline(&mut world);
}

#[test]
fn timestamp_offsets_zero_equal_to_current_and_greater_than_current_fail_closed() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    for invalid_offset in [0u64, 612_000u64, 612_001u64] {
        world
            .query()
            .to(PAIR)
            .returns(ExpectError(4, "Bad parameters"))
            .whitebox(pair::contract_obj, |sc| {
                sc.get_safe_price_by_timestamp_offset_ms(
                    PAIR.to_managed_address(),
                    invalid_offset,
                    EsdtTokenPayment::new(
                        managed_token_id!(WEGLD_TOKEN_ID),
                        0,
                        managed_biguint!(QUOTE_INPUT),
                    ),
                );
            });
    }
}

#[test]
fn equal_reversed_before_oldest_and_future_timestamp_ranges_fail_closed() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);

    for (start, end) in [(606_000u64, 606_000u64), (606_000u64, 600_000u64)] {
        world
            .query()
            .to(PAIR)
            .returns(ExpectError(4, "Bad parameters"))
            .whitebox(pair::contract_obj, |sc| {
                sc.get_safe_price_by_timestamp_range(
                    PAIR.to_managed_address(),
                    start,
                    end,
                    EsdtTokenPayment::new(
                        managed_token_id!(WEGLD_TOKEN_ID),
                        0,
                        managed_biguint!(QUOTE_INPUT),
                    ),
                );
            });
    }

    for (start, end) in [(599_400u64, 600_000u64), (612_000u64, 612_600u64)] {
        world
            .query()
            .to(PAIR)
            .returns(ExpectError(4, "The price observation does not exist"))
            .whitebox(pair::contract_obj, |sc| {
                sc.get_safe_price_by_timestamp_range(
                    PAIR.to_managed_address(),
                    start,
                    end,
                    EsdtTokenPayment::new(
                        managed_token_id!(WEGLD_TOKEN_ID),
                        0,
                        managed_biguint!(QUOTE_INPUT),
                    ),
                );
            });
    }
}

#[test]
fn legacy_seconds_offset_rejects_milliseconds_conversion_overflow_for_token_and_lp_views() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    seed_four_field_legacy_history_and_upgrade(&mut world);
    let overflowing_seconds = u64::MAX / 1_000u64 + 1u64;

    world
        .query()
        .to(PAIR)
        .returns(ExpectError(4, "Bad parameters"))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_safe_price_by_timestamp_offset(
                PAIR.to_managed_address(),
                overflowing_seconds,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(QUOTE_INPUT),
                ),
            );
        });
    world
        .query()
        .to(PAIR)
        .returns(ExpectError(4, "Bad parameters"))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_lp_tokens_safe_price_by_timestamp_offset(
                PAIR.to_managed_address(),
                overflowing_seconds,
                managed_biguint!(LP_QUOTE),
            );
        });
}

#[test]
fn default_offset_below_equal_and_above_available_history_uses_expected_clamped_window() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    let mut ledger = seed_four_field_legacy_history_and_upgrade(&mut world);

    set_block(&mut world, 103u64, 618_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        612_000u64,
        618_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );
    set_block(&mut world, 104u64, 624_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        618_000u64,
        624_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    world.block_round_time_ms(600u64);
    set_block(&mut world, 109u64, 627_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        624_000u64,
        627_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );
    set_block(&mut world, 114u64, 630_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        627_000u64,
        630_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );
    set_block(&mut world, 129u64, 639_000u64);
    swap_first_for_second(&mut world);
    ledger.push(
        630_000u64,
        639_000u64,
        INITIAL_FIRST_RESERVE,
        INITIAL_SECOND_RESERVE,
    );
    set_block(&mut world, 134u64, 642_000u64);
    swap_second_for_first(&mut world);
    ledger.push(
        639_000u64,
        642_000u64,
        INITIAL_SECOND_RESERVE,
        INITIAL_FIRST_RESERVE,
    );

    assert_default_offset_uses_expected_window(
        &mut world, &ledger, 24_000u64, 618_000u64, 642_000u64,
    );
    assert_default_offset_uses_expected_window(
        &mut world, &ledger, 42_000u64, 600_000u64, 642_000u64,
    );
    assert_default_offset_uses_expected_window(
        &mut world, &ledger, 60_000u64, 600_000u64, 642_000u64,
    );
}

#[test]
fn single_four_field_legacy_observation_upgrades_then_extends_with_native_post_supernova_history() {
    let mut world = setup_pair(6_000u64, 102u64, 612_000u64);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let mut legacy_observations =
                VecMapper::<DebugApi, LegacyPriceObservation<DebugApi>>::new(StorageKey::new(
                    b"price_observations",
                ));
            legacy_observations.push(&LegacyPriceObservation {
                first_token_reserve_accumulated: managed_biguint!(102_816_000u64),
                second_token_reserve_accumulated: managed_biguint!(205_632_000u64),
                weight_accumulated: 102u64,
                recording_round: 102u64,
            });
            sc.safe_price_current_index().set(1usize);
            sc.upgrade();
        });

    world.block_round_time_ms(600u64);
    set_block(&mut world, 103u64, 612_600u64);
    swap_first_for_second(&mut world);
    set_block(&mut world, 113u64, 618_600u64);
    swap_second_for_first(&mut world);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let legacy: PriceObservation<DebugApi> = sc.price_observations().get(1usize);
        let native = sc.price_observations().get(2usize);
        assert_eq!(legacy.recording_timestamp, 0u64);
        assert_eq!(legacy.lp_supply_accumulated, 0u64);
        assert_eq!(native.recording_timestamp, 618_600u64);
        assert!(native.lp_supply_accumulated > 0u64);
        assert_eq!(sc.safe_price_current_index().get(), 2usize);
        assert_eq!(
            sc.current_price_observation().get().recording_timestamp,
            618_600u64
        );
    });

    let average_first = (u128::from(INITIAL_FIRST_RESERVE) * 600u128
        + u128::from(INITIAL_SECOND_RESERVE) * 6_000u128)
        / 6_600u128;
    let average_second = (u128::from(INITIAL_SECOND_RESERVE) * 600u128
        + u128::from(INITIAL_FIRST_RESERVE) * 6_000u128)
        / 6_600u128;
    let expected = u64::try_from(u128::from(QUOTE_INPUT) * average_second / average_first).unwrap();
    assert_token_quote_by_timestamp_range(&mut world, 612_000u64, 618_600u64, expected);
}

#[test]
fn timestamp_interpolation_across_full_ring_physical_seam_current_index_one_is_correct() {
    let oldest_timestamp = 1_000u64;
    let oldest_round = 10_000u64;
    let penultimate_timestamp =
        oldest_timestamp + u64::try_from(MAX_OBSERVATIONS - 2usize).unwrap() * 600u64;
    let penultimate_round = oldest_round + u64::try_from(MAX_OBSERVATIONS - 2usize).unwrap();
    let latest_timestamp = penultimate_timestamp + 1_200u64;
    let latest_round = penultimate_round + 2u64;
    let midpoint_timestamp = penultimate_timestamp + 600u64;
    let midpoint_round = penultimate_round + 1u64;
    let penultimate_elapsed = penultimate_timestamp - oldest_timestamp;
    let penultimate_first_accumulated = penultimate_elapsed * 10u64;
    let penultimate_second_accumulated = penultimate_elapsed * 20u64;
    let penultimate_lp_accumulated = penultimate_elapsed * 100u64;
    let latest_first_accumulated = penultimate_first_accumulated + 1_200u64 * 30u64;
    let latest_second_accumulated = penultimate_second_accumulated + 1_200u64 * 50u64;
    let latest_lp_accumulated = penultimate_lp_accumulated + 1_200u64 * 70u64;
    let mut world = setup_pair(600u64, latest_round, latest_timestamp);

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let latest = PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(latest_first_accumulated),
                second_token_reserve_accumulated: managed_biguint!(latest_second_accumulated),
                weight_accumulated: latest_timestamp,
                recording_round: latest_round,
                recording_timestamp: latest_timestamp,
                lp_supply_accumulated: managed_biguint!(latest_lp_accumulated),
            };
            let historical_observation = |physical_index: usize| {
                let elapsed = u64::try_from(physical_index - 2usize).unwrap() * 600u64;
                let timestamp = oldest_timestamp + elapsed;
                PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(elapsed * 10u64),
                    second_token_reserve_accumulated: managed_biguint!(elapsed * 20u64),
                    weight_accumulated: timestamp,
                    recording_round: oldest_round + u64::try_from(physical_index - 2usize).unwrap(),
                    recording_timestamp: timestamp,
                    lp_supply_accumulated: managed_biguint!(elapsed * 100u64),
                }
            };

            sc.price_observations().push(&latest);
            for physical_index in 2usize..=MAX_OBSERVATIONS {
                sc.price_observations()
                    .push(&historical_observation(physical_index));
            }
            sc.safe_price_current_index().set(1usize);
            sc.current_price_observation().set(&latest);
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let midpoint = sc.get_price_observation_view(PAIR.to_managed_address(), midpoint_round);
        assert_eq!(midpoint.recording_round, midpoint_round);
        assert_eq!(midpoint.recording_timestamp, midpoint_timestamp);
        assert_eq!(midpoint.weight_accumulated, midpoint_timestamp);
        assert_eq!(
            midpoint.first_token_reserve_accumulated,
            managed_biguint!(penultimate_first_accumulated + 600u64 * 30u64)
        );
        assert_eq!(
            midpoint.second_token_reserve_accumulated,
            managed_biguint!(penultimate_second_accumulated + 600u64 * 50u64)
        );
        assert_eq!(
            midpoint.lp_supply_accumulated,
            managed_biguint!(penultimate_lp_accumulated + 600u64 * 70u64)
        );

        let token_quote = sc.get_safe_price_by_timestamp_range(
            PAIR.to_managed_address(),
            penultimate_timestamp,
            midpoint_timestamp,
            EsdtTokenPayment::new(
                managed_token_id!(WEGLD_TOKEN_ID),
                0,
                managed_biguint!(QUOTE_INPUT),
            ),
        );
        assert_eq!(
            token_quote.amount,
            managed_biguint!(QUOTE_INPUT * 50u64 / 30u64)
        );

        let (first_lp_quote, second_lp_quote) = sc
            .get_lp_tokens_safe_price_by_timestamp_range(
                PAIR.to_managed_address(),
                penultimate_timestamp,
                midpoint_timestamp,
                managed_biguint!(LP_QUOTE),
            )
            .into_tuple();
        assert_eq!(
            first_lp_quote.amount,
            managed_biguint!(LP_QUOTE * 30u64 / 70u64)
        );
        assert_eq!(
            second_lp_quote.amount,
            managed_biguint!(LP_QUOTE * 50u64 / 70u64)
        );
    });
}
