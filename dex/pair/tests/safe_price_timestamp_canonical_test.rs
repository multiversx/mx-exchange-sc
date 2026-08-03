use multiversx_sc::types::{TestAddress, TestSCAddress, TimestampMillis};
use multiversx_sc_scenario::imports::*;
use pair::{
    config::ConfigModule,
    safe_price::{PriceObservation, SafePriceModule},
    safe_price_view::SafePriceViewModule,
    Pair,
};

const OWNER: TestAddress = TestAddress::new("owner");
const PAIR: TestSCAddress = TestSCAddress::new("pair");
const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";

fn setup_empty_world(current_timestamp: u64) -> ScenarioWorld {
    let mut world = ScenarioWorld::debugger();
    world.register_contract("0x0500", pair::ContractBuilder);
    world
        .account(OWNER)
        .balance(0u64)
        .storage_mandos("str:default_safe_price_timestamp_offset", "u64:3600000")
        .storage_mandos("str:safe_price_timestamp_save_interval", "u64:6000");
    world
        .account(PAIR)
        .code(BytesValue::from_hex("0500"))
        .owner(OWNER)
        .balance(0u64);
    world.block_round_time_ms(600u64);
    world
        .current_block()
        .block_round(111u64)
        .block_timestamp_millis(TimestampMillis::new(current_timestamp));

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
            let second_token_id = managed_token_id!(MEX_TOKEN_ID);
            sc.init(
                first_token_id.clone(),
                second_token_id.clone(),
                OWNER.to_managed_address(),
                OWNER.to_managed_address(),
                300u64,
                50u64,
                ManagedAddress::zero(),
                MultiValueEncoded::new(),
            );

            sc.pair_reserve(&first_token_id)
                .set(managed_biguint!(10u64));
            sc.pair_reserve(&second_token_id)
                .set(managed_biguint!(20u64));
            sc.lp_token_supply().set(managed_biguint!(100u64));
        });

    world
}

fn setup_transition_world(current_timestamp: u64) -> ScenarioWorld {
    let mut world = setup_empty_world(current_timestamp);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(600_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_200_000u64),
                weight_accumulated: 600_000u64,
                recording_round: 100u64,
                recording_timestamp: 600_000u64,
                lp_supply_accumulated: managed_biguint!(6_000_000u64),
            });
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(633_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_365_000u64),
                weight_accumulated: 633_000u64,
                recording_round: 110u64,
                recording_timestamp: 633_000u64,
                lp_supply_accumulated: managed_biguint!(6_330_000u64),
            });
            sc.safe_price_current_index().set(2usize);
            sc.initialize_current_price_observation();
        });
    world
}

#[test]
fn test_fresh_post_supernova_oracle_uses_runtime_round_duration() {
    let mut world = setup_empty_world(606_600u64);

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.update_safe_price(
                &managed_biguint!(10u64),
                &managed_biguint!(20u64),
                &managed_biguint!(100u64),
            );
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert!(sc.price_observations().is_empty());
        assert_eq!(sc.safe_price_current_index().get(), 0usize);
        let current_observation = sc.current_price_observation().get();
        assert_eq!(current_observation.recording_round, 111u64);
        assert_eq!(current_observation.recording_timestamp, 606_600u64);
        assert_eq!(current_observation.weight_accumulated, 600u64);
    });

    world
        .current_block()
        .block_round(120u64)
        .block_timestamp_millis(TimestampMillis::new(612_000u64));
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.update_safe_price(
                &managed_biguint!(10u64),
                &managed_biguint!(20u64),
                &managed_biguint!(100u64),
            );
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        assert_eq!(sc.price_observations().len(), 1usize);
        assert_eq!(sc.safe_price_current_index().get(), 1usize);
        let finalized_observation = sc.price_observations().get(1usize);
        let current_observation = sc.current_price_observation().get();
        assert_eq!(finalized_observation.recording_round, 120u64);
        assert_eq!(finalized_observation.recording_timestamp, 612_000u64);
        assert_eq!(finalized_observation.weight_accumulated, 6_000u64);
        assert_eq!(
            current_observation.recording_timestamp,
            finalized_observation.recording_timestamp
        );
        assert_eq!(
            current_observation.weight_accumulated,
            finalized_observation.weight_accumulated
        );
    });
}

#[test]
fn test_safe_price_round_views_use_exact_protocol_timestamp_window() {
    let mut world = setup_transition_world(633_600u64);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let target_round = 107u64;
        let target_timestamp = 631_200u64;
        let current_round = 111u64;
        let current_timestamp = 633_600u64;
        let timestamp_offset = current_timestamp - target_timestamp;
        let pair_address = PAIR.to_managed_address();

        let target_observation = sc.get_price_observation_view(pair_address.clone(), target_round);
        assert_eq!(target_observation.recording_round, target_round);
        assert_eq!(target_observation.recording_timestamp, target_timestamp);

        let input_payment = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(100u64),
        );
        let timestamp_price = sc.get_safe_price_by_timestamp_offset(
            pair_address.clone(),
            timestamp_offset,
            input_payment.clone(),
        );
        assert_eq!(
            timestamp_price.token_identifier,
            managed_token_id!(MEX_TOKEN_ID)
        );
        assert_eq!(timestamp_price.amount, managed_biguint!(266u64));

        let round_price = sc.get_safe_price(
            pair_address.clone(),
            target_round,
            current_round,
            input_payment.clone(),
        );
        assert_eq!(round_price, timestamp_price);
        let round_offset_price = sc.get_safe_price_by_round_offset(
            pair_address.clone(),
            current_round - target_round,
            input_payment,
        );
        assert_eq!(round_offset_price, timestamp_price);

        let timestamp_lp_price = sc
            .get_lp_tokens_safe_price_by_timestamp_offset(
                pair_address.clone(),
                timestamp_offset,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(
            timestamp_lp_price.0.token_identifier,
            managed_token_id!(WEGLD_TOKEN_ID)
        );
        assert_eq!(timestamp_lp_price.0.amount, managed_biguint!(9u64));
        assert_eq!(
            timestamp_lp_price.1.token_identifier,
            managed_token_id!(MEX_TOKEN_ID)
        );
        assert_eq!(timestamp_lp_price.1.amount, managed_biguint!(25u64));

        let round_lp_price = sc
            .get_lp_tokens_safe_price(
                pair_address.clone(),
                target_round,
                current_round,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(round_lp_price, timestamp_lp_price);
        let round_offset_lp_price = sc
            .get_lp_tokens_safe_price_by_round_offset(
                pair_address,
                current_round - target_round,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(round_offset_lp_price, timestamp_lp_price);
    });
}

#[test]
fn test_all_new_post_supernova_history_has_zero_legacy_rounds() {
    let mut world = setup_empty_world(606_600u64);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(600_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_200_000u64),
                weight_accumulated: 600_000u64,
                recording_round: 100u64,
                recording_timestamp: 600_000u64,
                lp_supply_accumulated: managed_biguint!(6_000_000u64),
            });
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(660_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_320_000u64),
                weight_accumulated: 606_000u64,
                recording_round: 110u64,
                recording_timestamp: 606_000u64,
                lp_supply_accumulated: managed_biguint!(6_600_000u64),
            });
            sc.safe_price_current_index().set(2usize);
            sc.initialize_current_price_observation();
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let pair_address = PAIR.to_managed_address();
        let target_round = 107u64;
        let current_round = 111u64;
        let target_timestamp = 604_200u64;
        let timestamp_offset = 606_600u64 - target_timestamp;
        assert!(sc.safe_price_legacy_cutover().is_empty());

        let target_observation = sc.get_price_observation_view(pair_address.clone(), target_round);
        assert_eq!(target_observation.recording_round, target_round);
        assert_eq!(target_observation.recording_timestamp, target_timestamp);

        let input_payment = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(100u64),
        );
        let timestamp_price = sc.get_safe_price_by_timestamp_offset(
            pair_address.clone(),
            timestamp_offset,
            input_payment.clone(),
        );
        assert_eq!(timestamp_price.amount, managed_biguint!(200u64));
        let round_price = sc.get_safe_price(
            pair_address.clone(),
            target_round,
            current_round,
            input_payment,
        );
        assert_eq!(round_price, timestamp_price);

        let timestamp_lp_price = sc
            .get_lp_tokens_safe_price_by_timestamp_offset(
                pair_address.clone(),
                timestamp_offset,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(timestamp_lp_price.0.amount, managed_biguint!(10u64));
        assert_eq!(timestamp_lp_price.1.amount, managed_biguint!(20u64));
        let round_lp_price = sc
            .get_lp_tokens_safe_price(
                pair_address,
                target_round,
                current_round,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(round_lp_price, timestamp_lp_price);
    });
}

#[test]
fn test_exact_finalized_observation_is_available_when_pending_is_newer() {
    let mut world = setup_empty_world(606_600u64);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(600_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_200_000u64),
                weight_accumulated: 600_000u64,
                recording_round: 100u64,
                recording_timestamp: 600_000u64,
                lp_supply_accumulated: managed_biguint!(6_000_000u64),
            });
            sc.safe_price_current_index().set(1usize);
            sc.current_price_observation().set(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(660_000u64),
                second_token_reserve_accumulated: managed_biguint!(1_320_000u64),
                weight_accumulated: 606_000u64,
                recording_round: 110u64,
                recording_timestamp: 606_000u64,
                lp_supply_accumulated: managed_biguint!(6_600_000u64),
            });
        });

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let pair_address = PAIR.to_managed_address();
        let finalized = sc.get_price_observation_view(pair_address.clone(), 100u64);
        assert_eq!(finalized.recording_round, 100u64);
        assert_eq!(finalized.recording_timestamp, 600_000u64);
        assert_eq!(finalized.weight_accumulated, 600_000u64);

        let input_payment = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(100u64),
        );
        let safe_price =
            sc.get_safe_price(pair_address.clone(), 100u64, 111u64, input_payment.clone());
        assert_eq!(safe_price.amount, managed_biguint!(200u64));
        assert_eq!(
            sc.get_safe_price_by_default_offset(pair_address, input_payment),
            safe_price
        );

        let lp_safe_price = sc
            .get_lp_tokens_safe_price_by_default_offset(
                PAIR.to_managed_address(),
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(lp_safe_price.0.amount, managed_biguint!(10u64));
        assert_eq!(lp_safe_price.1.amount, managed_biguint!(20u64));
    });
}

#[test]
fn test_every_public_safe_price_view_and_legacy_compatibility_endpoint() {
    let mut world = setup_transition_world(633_600u64);

    world.query().to(PAIR).whitebox(pair::contract_obj, |sc| {
        let pair_address = PAIR.to_managed_address();
        let input_payment = EsdtTokenPayment::new(
            managed_token_id!(WEGLD_TOKEN_ID),
            0,
            managed_biguint!(100u64),
        );

        let explicit_price =
            sc.get_safe_price(pair_address.clone(), 107u64, 111u64, input_payment.clone());
        assert_eq!(
            sc.get_safe_price_by_round_offset(pair_address.clone(), 4u64, input_payment.clone(),),
            explicit_price
        );
        assert_eq!(
            sc.get_safe_price_by_timestamp_offset(
                pair_address.clone(),
                2_400u64,
                input_payment.clone(),
            ),
            explicit_price
        );

        let default_price =
            sc.get_safe_price_by_default_offset(pair_address.clone(), input_payment.clone());
        assert_eq!(
            default_price,
            sc.get_safe_price(pair_address.clone(), 100u64, 111u64, input_payment.clone(),)
        );
        assert_eq!(sc.update_and_get_safe_price(input_payment), default_price);

        let explicit_lp_price = sc
            .get_lp_tokens_safe_price(
                pair_address.clone(),
                107u64,
                111u64,
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(
            sc.get_lp_tokens_safe_price_by_round_offset(
                pair_address.clone(),
                4u64,
                managed_biguint!(100u64),
            )
            .into_tuple(),
            explicit_lp_price
        );
        assert_eq!(
            sc.get_lp_tokens_safe_price_by_timestamp_offset(
                pair_address.clone(),
                2_400u64,
                managed_biguint!(100u64),
            )
            .into_tuple(),
            explicit_lp_price
        );

        let default_lp_price = sc
            .get_lp_tokens_safe_price_by_default_offset(
                pair_address.clone(),
                managed_biguint!(100u64),
            )
            .into_tuple();
        assert_eq!(
            default_lp_price,
            sc.get_lp_tokens_safe_price(
                pair_address.clone(),
                100u64,
                111u64,
                managed_biguint!(100u64),
            )
            .into_tuple()
        );
        assert_eq!(
            sc.update_and_get_tokens_for_given_position_with_safe_price(managed_biguint!(100u64))
                .into_tuple(),
            default_lp_price
        );

        let observation = sc.get_price_observation_view(pair_address, 107u64);
        assert_eq!(observation.recording_round, 107u64);
        assert_eq!(observation.recording_timestamp, 631_200u64);
        assert_eq!(sc.safe_price_current_index().get(), 2usize);
        assert_eq!(
            sc.current_price_observation().get().recording_timestamp,
            633_000u64
        );
    });
}

#[test]
fn test_safe_price_views_reject_invalid_index_and_missing_current_observation() {
    for missing_current in [false, true] {
        let mut world = setup_transition_world(633_600u64);
        world
            .tx()
            .from(OWNER)
            .to(PAIR)
            .whitebox(pair::contract_obj, |sc| {
                if missing_current {
                    sc.current_price_observation().clear();
                } else {
                    sc.safe_price_current_index().set(3usize);
                }
            });

        let expected = if missing_current {
            "The price observation does not exist"
        } else {
            "Invalid safe price current index"
        };
        world
            .query()
            .to(PAIR)
            .returns(ExpectError(4, expected))
            .whitebox(pair::contract_obj, |sc| {
                sc.get_price_observation_view(PAIR.to_managed_address(), 107u64);
            });
    }
}

#[test]
fn test_safe_price_views_reject_timestamp_and_weight_regressions() {
    let mut timestamp_world = setup_transition_world(633_600u64);
    timestamp_world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let mut current = sc.current_price_observation().get();
            current.recording_timestamp = 632_999u64;
            sc.current_price_observation().set(&current);
        });
    timestamp_world
        .query()
        .to(PAIR)
        .returns(ExpectError(4, "Invalid safe price timestamp order"))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_price_observation_view(PAIR.to_managed_address(), 107u64);
        });

    let mut weight_world = setup_empty_world(606_600u64);
    weight_world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            for (round, timestamp, first, second) in [
                (100u64, 600_000u64, 1_000u64, 2_000u64),
                (110u64, 606_000u64, 1_100u64, 2_200u64),
            ] {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(first),
                    second_token_reserve_accumulated: managed_biguint!(second),
                    weight_accumulated: 100u64,
                    recording_round: round,
                    recording_timestamp: timestamp,
                    lp_supply_accumulated: managed_biguint!(1_000u64),
                });
            }
            sc.safe_price_current_index().set(2usize);
            sc.initialize_current_price_observation();
        });
    weight_world
        .query()
        .to(PAIR)
        .returns(ExpectError(
            4,
            "Safe price observations must have increasing cumulative weight",
        ))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_safe_price(
                PAIR.to_managed_address(),
                100u64,
                110u64,
                EsdtTokenPayment::new(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    0,
                    managed_biguint!(100u64),
                ),
            );
        });
}

#[test]
fn test_safe_price_writer_rejects_timestamp_regression() {
    let mut world = setup_transition_world(633_600u64);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            let mut current = sc.current_price_observation().get();
            current.recording_timestamp = 632_999u64;
            sc.current_price_observation().set(&current);
        });

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .returns(ExpectError(4, "Invalid safe price timestamp order"))
        .whitebox(pair::contract_obj, |sc| {
            sc.update_safe_price(
                &managed_biguint!(10u64),
                &managed_biguint!(20u64),
                &managed_biguint!(100u64),
            );
        });
}

#[test]
fn test_safe_price_upgrade_initialization_rejects_invalid_legacy_state() {
    for invalid_lp_accumulator in [false, true] {
        let mut world = setup_empty_world(660_000u64);
        world
            .tx()
            .from(OWNER)
            .to(PAIR)
            .whitebox(pair::contract_obj, |sc| {
                sc.price_observations().push(&PriceObservation {
                    first_token_reserve_accumulated: managed_biguint!(10u64),
                    second_token_reserve_accumulated: managed_biguint!(20u64),
                    weight_accumulated: 1u64,
                    recording_round: 100u64,
                    recording_timestamp: 0u64,
                    lp_supply_accumulated: if invalid_lp_accumulator {
                        managed_biguint!(1u64)
                    } else {
                        managed_biguint!(0u64)
                    },
                });
                sc.safe_price_current_index()
                    .set(if invalid_lp_accumulator {
                        1usize
                    } else {
                        2usize
                    });
                sc.safe_price_legacy_cutover().set((110u64, 660_000u64));
            });

        let expected = if invalid_lp_accumulator {
            "Cannot normalize legacy safe price observation"
        } else {
            "Invalid safe price current index"
        };
        world
            .tx()
            .from(OWNER)
            .to(PAIR)
            .returns(ExpectError(4, expected))
            .whitebox(pair::contract_obj, |sc| {
                sc.initialize_current_price_observation();
            });
    }
}

#[test]
fn test_safe_price_legacy_weight_scaling_overflow_fails_closed() {
    let mut world = setup_empty_world(660_000u64);
    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(10u64),
                second_token_reserve_accumulated: managed_biguint!(20u64),
                weight_accumulated: u64::MAX / 6_000u64 + 1u64,
                recording_round: 110u64,
                recording_timestamp: 0u64,
                lp_supply_accumulated: managed_biguint!(0u64),
            });
            sc.safe_price_current_index().set(1usize);
            sc.safe_price_legacy_cutover().set((110u64, 660_000u64));
        });

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .returns(ExpectError(4, "Safe price duration overflow"))
        .whitebox(pair::contract_obj, |sc| {
            sc.initialize_current_price_observation();
        });
}

#[test]
fn test_safe_price_round_view_rejects_non_protocol_timestamp_timeline() {
    let mut world = setup_transition_world(633_601u64);

    world
        .query()
        .to(PAIR)
        .returns(ExpectError(4, "The price observation does not exist"))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_price_observation_view(PAIR.to_managed_address(), 107u64);
        });
}

#[test]
fn test_post_supernova_legacy_history_without_cutover_fails_closed() {
    let mut world = setup_empty_world(633_600u64);

    world
        .tx()
        .from(OWNER)
        .to(PAIR)
        .whitebox(pair::contract_obj, |sc| {
            sc.price_observations().push(&PriceObservation {
                first_token_reserve_accumulated: managed_biguint!(100u64),
                second_token_reserve_accumulated: managed_biguint!(200u64),
                weight_accumulated: 10u64,
                recording_round: 100u64,
                recording_timestamp: 0u64,
                lp_supply_accumulated: managed_biguint!(0u64),
            });
            sc.safe_price_current_index().set(1usize);
        });

    world
        .query()
        .to(PAIR)
        .returns(ExpectError(
            4,
            "Cannot normalize legacy safe price observation",
        ))
        .whitebox(pair::contract_obj, |sc| {
            sc.get_price_observation_view(PAIR.to_managed_address(), 100u64);
        });
}
