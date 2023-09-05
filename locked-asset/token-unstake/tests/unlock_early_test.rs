#![allow(deprecated)]

mod token_unstake_setup;

use energy_factory::energy::EnergyModule;
use energy_query::Energy;
use multiversx_sc_scenario::{managed_address, managed_biguint, DebugApi};
use token_unstake_setup::*;

#[test]
fn double_unlock_early_test() {
    let _ = DebugApi::dummy();
    let mut setup =
        TokenUnstakeSetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    // lock for max period
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            USER_BALANCE,
            LOCK_OPTIONS[2],
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.energy_factory_wrapper, |sc| {
            let expected_energy_amount = managed_biguint!(LOCK_OPTIONS[2]) * USER_BALANCE;
            let expected_energy_entry = Energy::new(
                expected_energy_amount.into(),
                0,
                managed_biguint!(USER_BALANCE),
            );

            let actual_energy_entry = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy_entry, actual_energy_entry);
        })
        .assert_ok();

    setup.unlock_early(&first_user, 1, half_balance).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.energy_factory_wrapper, |sc| {
            let expected_energy_amount = managed_biguint!(LOCK_OPTIONS[2]) * half_balance;
            let expected_energy_entry = Energy::new(
                expected_energy_amount.into(),
                0,
                managed_biguint!(half_balance),
            );

            let actual_energy_entry = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy_entry, actual_energy_entry);
        })
        .assert_ok();

    setup.b_mock.set_block_epoch(UNBOND_EPOCHS);

    setup.unbond(&first_user).assert_ok();

    setup.unlock_early(&first_user, 1, half_balance).assert_ok();

    setup
        .b_mock
        .execute_query(&setup.energy_factory_wrapper, |sc| {
            let expected_energy_amount = managed_biguint!(0);
            let expected_energy_entry = Energy::new(
                expected_energy_amount.into(),
                UNBOND_EPOCHS,
                managed_biguint!(0),
            );

            let actual_energy_entry = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy_entry, actual_energy_entry);
        })
        .assert_ok();
}
