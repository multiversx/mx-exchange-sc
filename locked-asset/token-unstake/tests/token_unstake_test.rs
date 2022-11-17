mod token_unstake_setup;

use elrond_wasm::types::{EsdtTokenPayment, ManagedVec};
use elrond_wasm_debug::{
    managed_address, managed_token_id, managed_token_id_wrapped, rust_biguint, DebugApi,
};
use num_bigint::ToBigInt;
use num_traits::cast::ToPrimitive;
use simple_lock::locked_token::LockedTokenAttributes;
use token_unstake::{
    fees_merging::{EncodabLockedAmountWeightAttributesPair, FeesMergingModule},
    tokens_per_user::{TokensPerUserModule, UnstakePair},
};
use token_unstake_setup::*;

pub struct ResultWrapper<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    pub setup: TokenUnstakeSetup<EnergyFactoryBuilder, UnstakeScBuilder>,
    pub balance_after_second_reduce: num_bigint::BigUint,
    pub final_penalty_amount: num_bigint::BigUint,
}

#[test]
fn init_token_unstake_test() {
    let _ = TokenUnstakeSetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
}

#[test]
fn unstake_sc_fees_merging_and_unbond_test() {
    let result = unbond_test_common(energy_factory::contract_obj, token_unstake::contract_obj);
    let (mut setup, balance_after_second_reduce, final_penalty_amount) = (
        result.setup,
        result.balance_after_second_reduce,
        result.final_penalty_amount,
    );
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    // user try unbond early
    setup
        .unbond(&first_user)
        .assert_user_error("Nothing to unbond");

    // unbond epochs pass
    setup.b_mock.set_block_epoch(10 + UNBOND_EPOCHS);

    // unbond ok
    setup.unbond(&first_user).assert_ok();

    let final_user_balance = &balance_after_second_reduce - &final_penalty_amount;
    let user_balance_after_unbond = rust_biguint!(half_balance) + final_user_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &user_balance_after_unbond);

    // check fees added correctly
    // nonce is 3, since we already had a token with this unlock epoch
    setup.b_mock.check_nft_balance(
        &setup.fees_collector_mock,
        LOCKED_TOKEN_ID,
        3,
        &(final_penalty_amount / 2u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );

    let user_energy = setup.get_user_energy(&first_user);
    assert_eq!(user_energy, rust_biguint!(0));
}

#[test]
fn cancel_unbond_test() {
    let result = unbond_test_common(energy_factory::contract_obj, token_unstake::contract_obj);
    let (mut setup, balance_after_second_reduce, _final_penalty_amount) = (
        result.setup,
        result.balance_after_second_reduce,
        result.final_penalty_amount,
    );
    let first_user = setup.first_user.clone();

    setup.cancel_unbond(&first_user).assert_ok();

    // check user entries after unbond
    setup
        .b_mock
        .execute_query(&setup.unstake_sc_wrapper, |sc| {
            assert!(sc
                .unlocked_tokens_for_user(&managed_address!(&first_user))
                .is_empty());
        })
        .assert_ok();

    // check user balance - they get the locked token back
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &balance_after_second_reduce,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );

    // check energy was added back - current epoch is 10
    let user_energy = setup.get_user_energy(&first_user);
    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - 10) * balance_after_second_reduce;
    assert_eq!(user_energy, expected_energy);
}

fn unbond_test_common<EnergyFactoryBuilder, UnstakeScBuilder>(
    energy_factory_builder: EnergyFactoryBuilder,
    unstake_sc_builder: UnstakeScBuilder,
) -> ResultWrapper<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    let mut setup = TokenUnstakeSetup::new(energy_factory_builder, unstake_sc_builder);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    // lock for max period
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[2],
        )
        .assert_ok();

    // reduce lock period from 4 years to 2 years
    let penalty_percentage = 5_000u64; // (8_000 - 6_000) / (10_000 - 6_000) = 0.5 => 5_000
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, LOCK_OPTIONS[2], LOCK_OPTIONS[1]);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup
        .reduce_lock_period(&first_user, 1, half_balance, LOCK_OPTIONS[1])
        .assert_ok();

    let new_user_balance = rust_biguint!(half_balance) - &penalty_amount;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &new_user_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[1],
        }),
    );

    setup
        .b_mock
        .execute_query(&setup.unstake_sc_wrapper, |sc| {
            let actual_fees = sc.fees_from_penalty_unlocking().get();
            let expected_fees = EncodabLockedAmountWeightAttributesPair::<DebugApi> {
                // half is burned, half is kept as fees
                token_amount: to_managed_biguint(penalty_amount.clone() / 2u64),
                token_unlock_fee_percent: PENALTY_PERCENTAGES[2],
                attributes: LockedTokenAttributes::<DebugApi> {
                    original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
                    original_token_nonce: 0,
                    unlock_epoch: LOCK_OPTIONS[2],
                },
            };
            assert_eq!(actual_fees, expected_fees);
        })
        .assert_ok();

    let balance_u64 = new_user_balance
        .clone()
        .to_bigint()
        .unwrap()
        .to_u64()
        .unwrap();

    // reduce lock period from 2 years to 1 year
    let second_penalty_percentage = 3_333u64; // (6_000 - 4_000) / (10_000 - 4_000) = 0.33 => 3_333
    let second_expected_penalty_amount = &new_user_balance * &second_penalty_percentage / 10_000u64;
    let second_penalty_amount =
        setup.get_penalty_amount(balance_u64, LOCK_OPTIONS[1], LOCK_OPTIONS[0]);
    assert_eq!(second_penalty_amount, second_expected_penalty_amount);

    setup
        .reduce_lock_period(&first_user, 2, balance_u64, LOCK_OPTIONS[0])
        .assert_ok();

    let balance_after_second_reduce = new_user_balance - second_expected_penalty_amount;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &balance_after_second_reduce,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );

    // check merged fees
    let new_total_fees = penalty_amount / 2u64 + second_penalty_amount / 2u64;
    let new_fee_percent = 7_500; // weighted average over penalty of 8_000 and 6_000
    let new_unlock_epoch = 1_290; // from 7_500 penalty (of 8_000) max => max 1_440 to 1_290
    setup
        .b_mock
        .execute_query(&setup.unstake_sc_wrapper, |sc| {
            let actual_fees = sc.fees_from_penalty_unlocking().get();
            let expected_fees = EncodabLockedAmountWeightAttributesPair::<DebugApi> {
                // half is burned, half is kept as fees
                token_amount: to_managed_biguint(new_total_fees.clone()),
                token_unlock_fee_percent: new_fee_percent,
                attributes: LockedTokenAttributes::<DebugApi> {
                    original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
                    original_token_nonce: 0,
                    unlock_epoch: new_unlock_epoch,
                },
            };
            assert_eq!(actual_fees, expected_fees);
        })
        .assert_ok();

    // check fees merged and sent to fees collector after 1 week
    setup.b_mock.set_block_epoch(10);

    let new_amount_u64 = balance_after_second_reduce
        .clone()
        .to_bigint()
        .unwrap()
        .to_u64()
        .unwrap();

    // reduce lock period from 1 years to 0
    let final_penalty_percentage = 3_888u64; // 0 + (350 - 0) * (4_000 - 0) / (360 - 0) = 3_888
    let final_expected_penalty_amount =
        &balance_after_second_reduce * &final_penalty_percentage / 10_000u64;
    let final_penalty_amount = setup.get_penalty_amount(new_amount_u64, LOCK_OPTIONS[0] - 10, 0);
    assert_eq!(final_penalty_amount, final_expected_penalty_amount);

    setup
        .unlock_early(&first_user, 3, new_amount_u64)
        .assert_ok();

    // check user unbond entry
    let final_user_balance = &balance_after_second_reduce - &final_penalty_amount;
    setup
        .b_mock
        .execute_query(&setup.unstake_sc_wrapper, |sc| {
            let unbond_entries = sc
                .unlocked_tokens_for_user(&managed_address!(&first_user))
                .get();
            let expected_entries = ManagedVec::from_single_item(UnstakePair {
                locked_tokens: EsdtTokenPayment::new(
                    managed_token_id!(LOCKED_TOKEN_ID),
                    3,
                    to_managed_biguint(balance_after_second_reduce.clone()),
                ),
                unlocked_tokens: EsdtTokenPayment::new(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    0,
                    to_managed_biguint(final_user_balance.clone()),
                ),
                unlock_epoch: 10 + UNBOND_EPOCHS,
            });
            assert_eq!(unbond_entries, expected_entries);

            assert!(sc.fees_from_penalty_unlocking().is_empty());
        })
        .assert_ok();

    // check fees collector balance
    setup.b_mock.check_nft_balance(
        &setup.fees_collector_mock,
        LOCKED_TOKEN_ID,
        4,
        &new_total_fees,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    ResultWrapper {
        setup,
        balance_after_second_reduce,
        final_penalty_amount,
    }
}
