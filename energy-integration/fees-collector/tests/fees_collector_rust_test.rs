mod fees_collector_test_setup;

use elrond_wasm::types::ManagedVec;
use elrond_wasm_debug::{managed_address, managed_biguint, managed_token_id, rust_biguint};
use fees_collector::{
    fees_accumulation::{FeesAccumulationModule, TokenAmountPair},
    fees_splitting::FeesSplittingModule,
};
use fees_collector_test_setup::*;

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

    fc_setup.set_energy(&first_user, 1_000);
    fc_setup.set_energy(&second_user, 3_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user, 1).assert_ok();
    fc_setup.claim(&second_user, 1).assert_ok();

    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, FIRST_TOKEN_ID, &rust_zero);
    fc_setup
        .b_mock
        .check_esdt_balance(&second_user, FIRST_TOKEN_ID, &rust_zero);

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                managed_biguint!(USER_BALANCE)
            );
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(SECOND_TOKEN_ID))
                    .get(),
                managed_biguint!(USER_BALANCE / 2)
            );

            assert_eq!(sc.total_users_for_week(2).get(), 2);

            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 2)
                    .get(),
                managed_biguint!(3_000)
            );
            assert_eq!(sc.total_energy_for_week(2).get(), 4_000);
        })
        .assert_ok();

    // user try claim first week again
    fc_setup.claim(&first_user, 1).assert_ok();

    // state remains unchanged
    fc_setup
        .b_mock
        .check_esdt_balance(&first_user, FIRST_TOKEN_ID, &rust_zero);

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                managed_biguint!(USER_BALANCE)
            );
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(SECOND_TOKEN_ID))
                    .get(),
                managed_biguint!(USER_BALANCE / 2)
            );

            assert_eq!(sc.total_users_for_week(2).get(), 2);

            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                managed_biguint!(1_000)
            );
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 2)
                    .get(),
                managed_biguint!(3_000)
            );
            assert_eq!(sc.total_energy_for_week(2).get(), 4_000);
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

    fc_setup.set_energy(&first_user, 1_000);
    fc_setup.set_energy(&second_user, 3_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user, 1).assert_ok();
    fc_setup.claim(&second_user, 1).assert_ok();

    // advance week
    fc_setup.advance_week();

    fc_setup.claim(&first_user, 2).assert_ok();

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

            assert_eq!(expected_total_rewards, sc.total_rewards_for_week(2).get());
        })
        .assert_ok();

    let first_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 1_000u32 / 4_000u32;
    let first_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 1_000u32 / 4_000u32;

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
            // fees were cleared and accumulated in the total_rewards mapper
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(FIRST_TOKEN_ID))
                    .get(),
                managed_biguint!(0)
            );
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(SECOND_TOKEN_ID))
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
            assert_eq!(sc.total_rewards_for_week(2).get(), expected_total_rewards);

            // first user's energy was removed for this week, and added to week 3
            assert_eq!(sc.total_users_for_week(2).get(), 1);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                0
            );
            assert_eq!(sc.total_users_for_week(3).get(), 1);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 3)
                    .get(),
                1_000
            );
        })
        .assert_ok();

    // first user try claim again
    fc_setup.claim(&first_user, 2).assert_ok();

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
            assert_eq!(sc.total_rewards_for_week(2).get(), expected_total_rewards);

            // first user's energy was removed for this week, and added to week 3
            assert_eq!(sc.total_users_for_week(2).get(), 1);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 2)
                    .get(),
                0
            );
            assert_eq!(sc.total_users_for_week(3).get(), 1);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&first_user), 3)
                    .get(),
                1_000
            );
        })
        .assert_ok();

    // second user claim for week 2
    fc_setup.claim(&second_user, 2).assert_ok();

    let second_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 3_000u32 / 4_000u32;
    let second_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 3_000u32 / 4_000u32;

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

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // rewards are cleared after everyone claims
            assert!(sc.total_rewards_for_week(2).is_empty());
            assert_eq!(sc.total_users_for_week(2).get(), 0);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 2)
                    .get(),
                0
            );
            assert_eq!(sc.total_energy_for_week(2).get(), 0);

            assert_eq!(sc.total_users_for_week(3).get(), 2);
            assert_eq!(
                sc.user_energy_for_week(&managed_address!(&second_user), 3)
                    .get(),
                3_000
            );
        })
        .assert_ok();
}
