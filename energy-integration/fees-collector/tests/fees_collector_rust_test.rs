#![allow(deprecated)]

mod fees_collector_test_setup;

use energy_query::Energy;
use fees_collector::additional_locked_tokens::{AdditionalLockedTokensModule, BLOCKS_IN_WEEK};
use fees_collector::fees_accumulation::FeesAccumulationModule;
use fees_collector_test_setup::*;
use multiversx_sc::types::{BigInt, EsdtTokenPayment, ManagedVec};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    DebugApi,
};
use simple_lock::locked_token::LockedTokenAttributes;
use weekly_rewards_splitting::locked_token_buckets::LockedTokensBucket;
use weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule;
use weekly_rewards_splitting::{
    global_info::WeeklyRewardsGlobalInfo,
    locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule, ClaimProgress,
};

#[test]
fn setup_test() {
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);
    assert_eq!(fc_setup.get_current_week(), 1);

    fc_setup.advance_week();
    assert_eq!(fc_setup.get_current_week(), 2);
}

#[test]
fn claim_first_week_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

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
            let second_user_energy = Energy::new(
                BigInt::from(managed_biguint!(3_000)),
                current_epoch,
                managed_biguint!(500),
            );

            assert_eq!(sc.total_energy_for_week(1).get(), 4_000);
            assert_eq!(sc.total_locked_tokens_for_week(1).get(), 1_000);
            assert_eq!(sc.last_global_update_week().get(), 1);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 1
                }
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&second_user))
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
            let second_user_energy = Energy::new(
                BigInt::from(managed_biguint!(3_000)),
                current_epoch,
                managed_biguint!(500),
            );

            assert_eq!(sc.total_energy_for_week(1).get(), 4_000);
            assert_eq!(sc.total_locked_tokens_for_week(1).get(), 1_000);
            assert_eq!(sc.last_global_update_week().get(), 1);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 1
                }
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&second_user))
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
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

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
            // for 4 weeks inactive => global update
            // total -= 4 * 300 => 12_000 - 300 => 11_700
            //
            // second user update:
            // total -= 9_000 = 2_700 += 7_950 = 10_650
            //
            // first user update:
            // total -= 3_000 = 7_650 += 2_500

            assert_eq!(sc.total_energy_for_week(4).get(), 10_450); // 9_050
            assert_eq!(sc.total_locked_tokens_for_week(4).get(), 100);
            assert_eq!(sc.last_global_update_week().get(), 4);

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_500)),
                current_epoch,
                managed_biguint!(50),
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

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
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
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
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(1_000),
            );

            // 10_000 total prev week
            // first user's tokens get removed, as they expired
            // so we only decrease by second user's 500 tokens worth of energy
            //
            // - 7 * 500 global decrease (-3_500)
            // - 1_000 (first user's surplus energy)
            // + 2_000 (first user's new energy)
            // = 7_500
            assert_eq!(sc.total_energy_for_week(2).get(), 7_500);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 1_500);
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(1_000),
            );

            assert_eq!(sc.total_energy_for_week(2).get(), 7_500);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 1_500);
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

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
            assert_eq!(sc.total_energy_for_week(2).get(), 11_300); // 11_650
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 100);
            assert_eq!(sc.last_global_update_week().get(), 2);

            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_650)),
                current_epoch,
                managed_biguint!(50),
            );
            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
fn try_claim_after_unlock() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

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
                sc.current_claim_progress(&managed_address!(&first_user))
                    .get(),
                ClaimProgress {
                    energy: first_user_energy,
                    week: 2
                }
            );
        })
        .assert_ok();
}

#[test]
fn locked_token_buckets_shifting_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

    fc_setup.set_energy(&first_user, 50, 3_000);
    fc_setup.set_energy(&second_user, 50, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.first_bucket_id().get(), 0);
            assert_eq!(
                sc.total_locked_tokens_for_week(1).get(),
                managed_biguint!(100)
            );

            // first user energy lasts for 3_000 / 50 = 60 epochs => 60 / 7 weeks to expire
            // => bucket offset 8
            // => surplus = 3_000 % (50 * 7) = 3_000 % 350 = 200
            //
            // second user energy lasts for 9_000 / 50 = 180 epochs => 180 / 7 weeks to expire
            // => bucket offset 25
            // => surplus = 9_000 % (50 * 7) = 9_000 % 350 = 250
            for i in 0..8 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(8).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(50),
                    surplus_energy_amount: managed_biguint!(200)
                }
            );

            for i in 9..25 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(25).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(50),
                    surplus_energy_amount: managed_biguint!(250)
                }
            );
        })
        .assert_ok();

    // advance week
    fc_setup.advance_week();

    // deposit rewards week 2
    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // advance 5 weeks
    fc_setup.advance_week();
    fc_setup.advance_week();
    fc_setup.advance_week();
    fc_setup.advance_week();
    fc_setup.advance_week();

    // naturally, energy would decrease with 7 * 5 * 50 = 1_750 =>
    // new energy = 3_000 - 1_750 = 1_250
    fc_setup.set_energy(&first_user, 50, 1_250);

    // let's assume second user locked some more tokens, and now has more energy
    fc_setup.set_energy(&second_user, 100, 10_000);

    // first user claim, which triggers the global update
    // and updates first user amounts
    fc_setup.claim(&first_user).assert_ok();

    // check internal storage after shift
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            // 6 weeks have passed, so we must shift 6 times (first bucket ID was 0 initially)
            assert_eq!(sc.first_bucket_id().get(), 6);
            assert_eq!(
                sc.total_locked_tokens_for_week(7).get(),
                managed_biguint!(100)
            );

            // first user energy lasts for 1_250 / 50 = 25 epochs => 3 weeks => offset 3
            // => bucket ID 6 + 3 = 9
            // second user did not update, so they remain in bucket 25

            // buckets shift 6 to the left
            for i in 6..8 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(8).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(0),
                    surplus_energy_amount: managed_biguint!(0)
                }
            );
            assert_eq!(
                sc.locked_tokens_in_bucket(9).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(50),
                    surplus_energy_amount: managed_biguint!(200)
                }
            );

            for i in 10..25 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(25).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(50),
                    surplus_energy_amount: managed_biguint!(250)
                }
            );
        })
        .assert_ok();

    // second user claim, which updates second user's amounts
    fc_setup.claim(&second_user).assert_ok();
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.first_bucket_id().get(), 6);
            assert_eq!(
                sc.total_locked_tokens_for_week(7).get(),
                managed_biguint!(150)
            );

            // second user energy lasts for 10_000 / 100 = 100 => 14 weeks => offset 14
            // => bucket ID = 6 + 14 = 20
            // => surplus = 10_000 % (7 * 100) = 10_000 % 700 = 200

            for i in 6..8 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(8).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(0),
                    surplus_energy_amount: managed_biguint!(0)
                }
            );
            assert_eq!(
                sc.locked_tokens_in_bucket(9).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(50),
                    surplus_energy_amount: managed_biguint!(200)
                }
            );

            for i in 10..20 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(20).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(100),
                    surplus_energy_amount: managed_biguint!(200)
                }
            );

            for i in 21..25 {
                assert!(sc.locked_tokens_in_bucket(i).is_empty());
            }
            assert_eq!(
                sc.locked_tokens_in_bucket(25).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(0),
                    surplus_energy_amount: managed_biguint!(0)
                }
            );
        })
        .assert_ok();
}

#[test]
fn multi_bucket_shift_consistency_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

    fc_setup.set_energy(&first_user, 1_000, 7_000);
    fc_setup.set_energy(&second_user, 100, 2_100);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();

    // user claim first week - users only get registered for week 2, without receiving rewards
    fc_setup.claim(&first_user).assert_ok();
    fc_setup.claim(&second_user).assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.first_bucket_id().get(), 0);
            assert_eq!(
                sc.total_locked_tokens_for_week(1).get(),
                managed_biguint!(1_100)
            );

            assert!(sc.locked_tokens_in_bucket(0).is_empty());
            assert_eq!(
                sc.locked_tokens_in_bucket(1).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(1_000),
                    surplus_energy_amount: managed_biguint!(0) // 7_000 % (7 * 1_000)
                }
            );
            assert!(sc.locked_tokens_in_bucket(2).is_empty());
            assert_eq!(
                sc.locked_tokens_in_bucket(3).get(),
                LockedTokensBucket::<DebugApi> {
                    token_amount: managed_biguint!(100),
                    surplus_energy_amount: managed_biguint!(0) // 2_100 % (7 * 100)
                }
            );
        })
        .assert_ok();

    // advance two weeks
    fc_setup.advance_week();
    fc_setup.advance_week();

    // check internal storage after shift
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            sc.perform_weekly_update(3);

            // 2 weeks passed since last update, so first_bucket_id => 0 + 2
            // first user's tokens were shifted out, only second user remains
            assert_eq!(sc.first_bucket_id().get(), 2);
            assert_eq!(sc.total_locked_tokens_for_week(3).get(), 100u64);

            // for week 2: energy -= 7 * 1_100 => 9_100 - 7_700 => 1_400
            // for week 3: energy -= 7 * 100 => 1_400 - 700 => 700
            assert_eq!(sc.total_energy_for_week(3).get(), 700u64);
        })
        .assert_ok();
}

#[test]
fn surplus_energy_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    // first user, 7_500 energy, 1_000 tokens (7 epochs, 1 week)
    // => bucket offset 1, surplus = 500

    // second user, 15_000 energy, 1_000 tokens (14 epochs, 2 week)
    // => bucket offset 1, surplus = 1_000

    // third user, 20_100 energy, 500 tokens (40 epochs => 5 weeks)
    // => bucket offset 2, surplus = 2_600

    fc_setup
        .b_mock
        .execute_tx(
            &fc_setup.owner_address,
            &fc_setup.fc_wrapper,
            &rust_zero,
            |sc| {
                sc.reallocate_bucket_after_energy_update(
                    &Energy::default(),
                    &Energy::default(),
                    &Energy::new(
                        managed_biguint!(7_500).into(),
                        INIT_EPOCH,
                        managed_biguint!(1_000),
                    ),
                );

                sc.reallocate_bucket_after_energy_update(
                    &Energy::default(),
                    &Energy::default(),
                    &Energy::new(
                        managed_biguint!(15_000).into(),
                        INIT_EPOCH,
                        managed_biguint!(1_000),
                    ),
                );

                sc.reallocate_bucket_after_energy_update(
                    &Energy::default(),
                    &Energy::default(),
                    &Energy::new(
                        managed_biguint!(20_100).into(),
                        INIT_EPOCH,
                        managed_biguint!(500),
                    ),
                );

                assert_eq!(
                    sc.locked_tokens_in_bucket(1).get(),
                    LockedTokensBucket::<DebugApi> {
                        token_amount: managed_biguint!(1_000),
                        surplus_energy_amount: managed_biguint!(500)
                    }
                );
                assert_eq!(
                    sc.locked_tokens_in_bucket(2).get(),
                    LockedTokensBucket::<DebugApi> {
                        token_amount: managed_biguint!(1_000),
                        surplus_energy_amount: managed_biguint!(1_000)
                    }
                );
                assert_eq!(
                    sc.locked_tokens_in_bucket(5).get(),
                    LockedTokensBucket::<DebugApi> {
                        token_amount: managed_biguint!(500),
                        surplus_energy_amount: managed_biguint!(2_600)
                    }
                );

                let mut total_energy = managed_biguint!(7_500u64 + 15_000u64 + 20_100u64);
                let mut total_tokens = managed_biguint!(1_000u64 + 1_000u64 + 500u64);
                assert_eq!(total_energy, 42_600u64);
                assert_eq!(total_tokens, 2_500u64);

                // no token shifted out yet, as first_token_id = 0
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 25_100u64); // 42_600 - 7 * 2_500
                assert_eq!(total_tokens, 2_500u64);

                // first bucket gets shifted
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 14_100u64); // 25_100 - 7 * 1_500 - 500
                assert_eq!(total_tokens, 1_500u64);

                // second bucket gets shifted
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 9_600u64); // 14_100 - 7 * 500 - 1_000
                assert_eq!(total_tokens, 500u64);

                // no shift
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 6_100u64); // 9_600 - 7 * 500
                assert_eq!(total_tokens, 500u64);

                // no shift
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 2_600u64); // 6_100 - 7 * 500
                assert_eq!(total_tokens, 500u64);

                // last bucket shift
                sc.shift_buckets_and_update_tokens_energy(1, &mut total_tokens, &mut total_energy);
                assert_eq!(total_energy, 0u64); // 2_600 - 7 * 0 - 2_600
                assert_eq!(total_tokens, 0u64);
            },
        )
        .assert_ok();
}

#[test]
fn claim_locked_rewards_with_energy_update_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    let first_user = fc_setup.b_mock.create_user_account(&rust_zero);
    let second_user = fc_setup.b_mock.create_user_account(&rust_zero);

    fc_setup.set_energy(&first_user, 500, 1_000);
    fc_setup.set_energy(&second_user, 500, 9_000);

    fc_setup.deposit(FIRST_TOKEN_ID, USER_BALANCE).assert_ok();
    fc_setup
        .deposit(SECOND_TOKEN_ID, USER_BALANCE / 2)
        .assert_ok();
    fc_setup
        .deposit_locked_tokens(LOCKED_TOKEN_ID, 1, USER_BALANCE / 100)
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
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(LOCKED_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 100),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
            assert_eq!(expected_total_rewards, sc.total_rewards_for_week(1).get());
        })
        .assert_ok();

    let first_user_expected_first_token_amt = rust_biguint!(USER_BALANCE) * 1_000u32 / 10_000u32;
    let first_user_expected_second_token_amt =
        rust_biguint!(USER_BALANCE / 2) * 1_000u32 / 10_000u32;
    let first_user_expected_locked_token_amt =
        rust_biguint!(USER_BALANCE / 100) * 1_000u32 / 10_000u32;

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
    fc_setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &first_user_expected_locked_token_amt,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 1440,
        }),
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
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(LOCKED_TOKEN_ID))
                    .get(),
                managed_biguint!(0)
            );

            let mut expected_total_rewards = ManagedVec::new();
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(LOCKED_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 100),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(2_000)),
                current_epoch,
                managed_biguint!(1_000),
            );

            // 10_000 total prev week
            // first user's tokens get removed, as they expired
            // so we only decrease by second user's 500 tokens worth of energy
            //
            // - 7 * 500 global decrease (-3_500)
            // - 1_000 (first user's surplus energy)
            // + 2_000 (first user's new energy)
            // = 7_500
            assert_eq!(sc.total_energy_for_week(2).get(), 7_500);
            assert_eq!(sc.total_locked_tokens_for_week(2).get(), 1_500);
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
    fc_setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &first_user_expected_locked_token_amt,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 1440,
        }),
    );

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            let mut expected_total_rewards = ManagedVec::new();
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(LOCKED_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 100),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(FIRST_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE),
            ));
            expected_total_rewards.push(EsdtTokenPayment::new(
                managed_token_id!(SECOND_TOKEN_ID),
                0,
                managed_biguint!(USER_BALANCE / 2),
            ));
            assert_eq!(sc.total_rewards_for_week(1).get(), expected_total_rewards);

            // first user's new energy is added to week 2
            // added energy: 1440 (unlock epoch) - 12 (curent epoch) = 1428 * 1000000000000000 = 1428000000000000000
            // final energy: 1428 * 1000000000000000 + 2000 (initial energy)
            let first_user_energy = Energy::new(
                BigInt::from(managed_biguint!(1428000000000002000u64)),
                current_epoch,
                managed_biguint!(1000000000001000u64),
            );

            // total initial energy: 7500
            // total updated energy: 100 (unlock epoch) - 12 (curent epoch) = 1428 * 1000000000000000 = 1428000000000000000
            assert_eq!(
                sc.total_energy_for_week(2).get(),
                managed_biguint!(1428000000000007500u64)
            );
            assert_eq!(
                sc.total_locked_tokens_for_week(2).get(),
                1000000000001500u64
            ); // 1000000000000000 + 1500
            assert_eq!(sc.last_global_update_week().get(), 2);

            assert_eq!(
                sc.current_claim_progress(&managed_address!(&first_user))
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
fn additional_locked_tokens_test() {
    let rust_zero = rust_biguint!(0);
    let mut fc_setup =
        FeesCollectorSetup::new(fees_collector::contract_obj, energy_factory::contract_obj);

    fc_setup.advance_week(); //current_week = 1

    fc_setup
        .b_mock
        .execute_tx(
            &fc_setup.owner_address,
            &fc_setup.fc_wrapper,
            &rust_zero,
            |sc| {
                sc.set_locked_tokens_per_block(managed_biguint!(1_000));
            },
        )
        .assert_ok();

    // nothing accumulated yet, as locked_tokens_per_block was 0
    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.last_locked_token_add_week().get(), 2);
            assert_eq!(sc.locked_tokens_per_block().get(), 1_000u64);
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(LOCKED_TOKEN_ID))
                    .get(),
                0u64
            );
        })
        .assert_ok();

    // cumulating again on same week does nothing

    fc_setup
        .b_mock
        .execute_tx(
            &fc_setup.owner_address,
            &fc_setup.fc_wrapper,
            &rust_zero,
            |sc| {
                sc.accumulate_additional_locked_tokens();
            },
        )
        .assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.last_locked_token_add_week().get(), 2);
            assert_eq!(sc.locked_tokens_per_block().get(), 1_000u64);
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(LOCKED_TOKEN_ID))
                    .get(),
                0u64
            );
        })
        .assert_ok();

    // cumulate on next week - tokens are allocated in current_week (2) - 1
    fc_setup.advance_week(); //current_week = 2

    fc_setup
        .b_mock
        .execute_tx(
            &fc_setup.owner_address,
            &fc_setup.fc_wrapper,
            &rust_zero,
            |sc| {
                sc.accumulate_additional_locked_tokens();
            },
        )
        .assert_ok();

    fc_setup
        .b_mock
        .execute_query(&fc_setup.fc_wrapper, |sc| {
            assert_eq!(sc.last_locked_token_add_week().get(), 3);
            assert_eq!(sc.locked_tokens_per_block().get(), 1_000u64);
            assert_eq!(
                sc.accumulated_fees(2, &managed_token_id!(LOCKED_TOKEN_ID))
                    .get(),
                BLOCKS_IN_WEEK * 1_000u64
            );
        })
        .assert_ok();
}
