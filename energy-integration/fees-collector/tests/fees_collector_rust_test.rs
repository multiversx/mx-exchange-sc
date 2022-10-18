mod fees_collector_test_setup;

use common_types::TokenAmountPair;
use elrond_wasm::{
    elrond_codec::multi_types::OptionalValue,
    types::{BigInt, ManagedVec, MultiValueEncoded, OperationCompletionStatus},
};
use elrond_wasm_debug::{managed_address, managed_biguint, managed_token_id, rust_biguint};
use elrond_wasm_modules::pause::PauseModule;
use energy_query::Energy;
use fees_collector::{fees_accumulation::FeesAccumulationModule, FeesCollector};
use fees_collector_test_setup::*;
use weekly_rewards_splitting::{ClaimProgress, WeeklyRewardsSplittingModule};

#[test]
fn setup_test() {
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );
    assert_eq!(fc_setup.get_current_week(), 1);

    fc_setup.advance_week();
    assert_eq!(fc_setup.get_current_week(), 2);
}

#[test]
fn claim_first_week_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 500, 1_000);
    fc_setup.set_energy(&second_user, 500, 3_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, FIRST_TOKEN_ID, &rust_zero);
    fc_setup
        .b_mock
        .check_esdt_balance(&second_user, FIRST_TOKEN_ID, &rust_zero);

    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                USER_BALANCE
            );
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(SECOND_TOKEN_ID))
                    .get(),
                USER_BALANCE / 2
            );

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(1_000)),
                current_epoch,
                managed_biguint!(500),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 1)
                    .get(),
                first_user_energy
            );

            let second_user_energy = Energy::new(
                BigInt::from(managed_biguint!(3_000)),
                current_epoch,
                managed_biguint!(500),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 1)
                    .get(),
                second_user_energy
            );

            assert_eq!(sc.total_energy_for_week(1).get(), 4_000);
            assert_eq!(sc.total_locked_tokens_for_week(1).get(), 1_000);
            assert_eq!(sc.last_global_update_week().get(), 1);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 1
                }
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&second_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: second_user_energy,
                    week: 1
                }
            );
        })
        .assert_ok();

    // user try claim first week again
    fc_setup.claim(&first_user).assert_ok();

    // state remains unchanged
    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, FIRST_TOKEN_ID, &rust_zero);

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                USER_BALANCE
            );
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(SECOND_TOKEN_ID))
                    .get(),
                USER_BALANCE / 2
            );

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(1_000)),
                current_epoch,
                managed_biguint!(500),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 1)
                    .get(),
                first_user_energy
            );

            let second_user_energy = Energy::new(
                BigInt::from(managed_biguint!(3_000)),
                current_epoch,
                managed_biguint!(500),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 1)
                    .get(),
                second_user_energy
            );

            assert_eq!(sc.total_energy_for_week(1).get(), 4_000);
            assert_eq!(sc.total_locked_tokens_for_week(1).get(), 1_000);
            assert_eq!(sc.last_global_update_week().get(), 1);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 1
                }
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&second_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: second_user_energy,
                    week: 1
                }
            );
        })
        .assert_ok();
}

#[test]
fn claim_after_dex_inactive_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 50, 3_000);
    fc_setup.set_energy(&second_user, 50, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    // advance to week 2 (inactive week)
    fc_setup.advance_week();

    // advance to week 3 (inactive week)
    fc_setup.advance_week();

    // advance to week 4 (active week)
    fc_setup.advance_week();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // Current week = 1; previous week = 0;
            assert_eq!(sc.current_global_active_week().get(), 1);
            assert_eq!(sc.last_global_active_week().get(), 0);
        })
        .assert_ok();

    // deposit rewards week 4
    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // decrease user energy
    fc_setup.set_energy(&first_user, 50, 2_500);

    // users claims in week 4
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // Current week = 4; previous week = 1;
            assert_eq!(sc.current_global_active_week().get(), 4);
            assert_eq!(sc.last_global_active_week().get(), 1);
        })
        .assert_ok();

    let first_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 3_000u32 / 12_000u32;
    let first_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 3_000u32 / 12_000u32;

    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        FIRST_TOKEN_ID,
        &first_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        SECOND_TOKEN_ID,
        &first_user_expected_second_token_amt,
    );

    // energy week 4 for second user will be 9_000 - 7 * 3 * 50 = 9_000 - 1_050 = 7_950
    let second_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 9_000u32 / 12_000u32;
    let second_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 9_000u32 / 12_000u32;

    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        FIRST_TOKEN_ID,
        &second_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        SECOND_TOKEN_ID,
        &second_user_expected_second_token_amt,
    );

    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.total_energy_for_week(4).get(), 10_450);
            assert_eq!(sc.total_locked_tokens_for_week(4).get(), 100);
            assert_eq!(sc.last_global_update_week().get(), 4);

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_500)),
                current_epoch,
                managed_biguint!(50),
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 4
                }
            );
        })
        .assert_ok();
}

#[test]
fn claim_second_week_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 500, 1_000);
    fc_setup.set_energy(&second_user, 500, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    // advance week
    fc_setup.advance_week();

    // increase first user's energy
    fc_setup.set_energy(&first_user, 1000, 2_000);

    // claim week 2 - receives rewards accumulated in week 1, and gets new energy saved
    fc_setup.claim(&first_user).assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            let mut expected_total_rewards = ManagedVec::new();
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(FIRST_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE),
            });
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(SECOND_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE / 2),
            });

            assert_eq!(expected_total_rewards, sc.total_rewards_for_week(1).get());
        })
        .assert_ok();

    let first_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 1_000u32 / 10_000u32;
    let first_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 1_000u32 / 10_000u32;

    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        FIRST_TOKEN_ID,
        &first_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        SECOND_TOKEN_ID,
        &first_user_expected_second_token_amt,
    );

    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // fees were cleared and accumulated in the total_rewards mapper
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                managed_biguint!(0)
            );
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(SECOND_TOKEN_ID))
                    .get(),
                managed_biguint!(0)
            );

            let mut expected_total_rewards = ManagedVec::new();
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(FIRST_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE),
            });
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(SECOND_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE / 2),
            });
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(1_000),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                first_user_energy
            );

            // 10_000 total prev week
            // - 7 * 1_000 global decrease (-7_000)
            // + 7 * 500 - 1_000 + 2_000 (+4_500) (for first user -> global refund + energy update)
            // = 10_000 - 7_000 + 4_500 = 7_500
            assert_eq!(sc.total_energy_for_week(2).get(), 7_500);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 1_500);
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 2
                }
            );
        })
        .assert_ok();

    // first user try claim again
    fc_setup.claim(&first_user).assert_ok();

    // no rewards were given, and state remains intact
    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        FIRST_TOKEN_ID,
        &first_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        SECOND_TOKEN_ID,
        &first_user_expected_second_token_amt,
    );

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            let mut expected_total_rewards = ManagedVec::new();
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(FIRST_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE),
            });
            expected_total_rewards.push(TokenAmountPair {
                token: managed_token_id!(SECOND_TOKEN_ID),
                amount: managed_biguint!(USER_BALANCE / 2),
            });
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(1_000),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                first_user_energy
            );

            assert_eq!(sc.total_energy_for_week(2).get(), 7_500);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 1_500);
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 2
                }
            );
        })
        .assert_ok();

    // second user claim for week 2
    fc_setup.claim(&second_user).assert_ok();

    let second_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 9_000u32 / 10_000u32;
    let second_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 9_000u32 / 10_000u32;

    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        FIRST_TOKEN_ID,
        &second_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        SECOND_TOKEN_ID,
        &second_user_expected_second_token_amt,
    );
}

#[test]
fn claim_inactive_week_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 50, 3_000);
    fc_setup.set_energy(&second_user, 50, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    // advance week
    fc_setup.advance_week();

    // deposit rewards week 2
    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // decrease user energy
    fc_setup.set_energy(&first_user, 50, 2_650);

    // only first user claims in second week
    fc_setup.claim(&first_user).assert_ok();

    let first_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 3_000u32 / 12_000u32;
    let first_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 3_000u32 / 12_000u32;

    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        FIRST_TOKEN_ID,
        &first_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &first_user,
        SECOND_TOKEN_ID,
        &first_user_expected_second_token_amt,
    );

    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // 12_000 - 700 + 350 - 3_000 + 2_650
            // = 11_300 + 350 - 350
            // = 11_300
            assert_eq!(sc.total_energy_for_week(2).get(), 11_300);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 100);
            assert_eq!(sc.last_global_update_week().get(), 2);

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_650)),
                current_epoch,
                managed_biguint!(50),
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 2
                }
            );
        })
        .assert_ok();

    // advance week
    fc_setup.advance_week();

    // second user claim third week
    fc_setup.claim(&second_user).assert_ok();

    // energy week 2 for second user will be 9_000 - 7 * 50 = 9_000 - 350 = 8_650
    let second_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 9_000u32 / 12_000u32
        + rust_biguint!(USER_BALANCE) * 8_650u32 / 11_300u32;
    let second_user_expected_second_token_amt = rust_biguint!(USER_BALANCE / 2) * 9_000u32
        / 12_000u32
        + rust_biguint!(USER_BALANCE / 2) * 8_650u32 / 11_300u32;

    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        FIRST_TOKEN_ID,
        &second_user_expected_first_token_amt,
    );
    fc_setup.b_mock.check_esdt_balance(
        &second_user,
        SECOND_TOKEN_ID,
        &second_user_expected_second_token_amt,
    );
}

#[test]
fn owner_update_energy_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 50, 3_000);
    fc_setup.set_energy(&second_user, 50, 9_000);

    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    let owner = fc_setup.owner_address.clone();
    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_tx(&owner, &fc_setup.fc_wrapper, &rust_zero, |sc| {
            sc.set_paused(true);

            let mut args = MultiValueEncoded::new();
            args.push(
                (
                    managed_address!(&first_user),
                    managed_biguint!(2_000),
                    managed_biguint!(45),
                )
                    .into(),
            );

            let (status, opt_index) = sc.recompute_energy(args).into_tuple();
            assert!(matches!(status, OperationCompletionStatus::Completed));
            assert!(matches!(opt_index, OptionalValue::None));

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(45),
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 1)
                    .get(),
                first_user_energy
            );

            assert_eq!(sc.total_energy_for_week(1).get(), 11_000);
            assert_eq!(sc.total_locked_tokens_for_week(1).get(), 95);
            assert_eq!(sc.last_global_update_week().get(), 1);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 1
                }
            );
        })
        .assert_ok();
}

#[test]
fn try_claim_after_unlock() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup = FeesCollectorSetup::new(
        fees_collector::contract_obj,
        energy_factory_mock::contract_obj,
    );

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let fees_collector_nonce = 0u64;

    fc_setup.set_energy(&first_user, 50, 3_000);
    fc_setup.set_energy(&second_user, 50, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    // advance week
    fc_setup.advance_week();

    // deposit rewards week 2
    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // decrease user energy
    fc_setup.set_energy(&first_user, 50, 1_000);

    // only first user claims in second week
    fc_setup.claim(&first_user).assert_ok();

    // no rewards are received, as energy decreased from the calculated amount
    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, FIRST_TOKEN_ID, &rust_zero);
    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, SECOND_TOKEN_ID, &rust_zero);

    let current_epoch = fc_setup.current_epoch;
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(1_000)),
                current_epoch,
                managed_biguint!(50),
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user), fees_collector_nonce)
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 2
                }
            );
        })
        .assert_ok();
}
