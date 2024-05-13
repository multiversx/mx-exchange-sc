#![allow(deprecated)]

mod farm_setup;

use common_structs::FarmTokenAttributes;
use farm::Farm;
use farm_setup::multi_user_farm_setup::*;
use multiversx_sc::imports::OptionalValue;
use multiversx_sc_scenario::{
    managed_biguint, rust_biguint, whitebox_legacy::TxTokenTransfer, DebugApi,
};
use weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo;

#[test]
fn test_farm_setup() {
    let _ = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );
}

#[test]
fn test_energy_update() {
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    let energy_amount = 1_000;
    farm_setup.set_user_energy(&first_user, energy_amount, 13, 1);
    farm_setup.check_farm_claim_progress_energy(0);

    farm_setup.update_energy_for_user();
    farm_setup.check_farm_claim_progress_energy(energy_amount);
}

#[test]
fn test_energy_update_no_claim_current_week() {
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    farm_setup.b_mock.set_block_epoch(5);
    farm_setup.update_energy_for_user();

    farm_setup.b_mock.set_block_epoch(15);

    farm_setup.update_energy_for_user();
    farm_setup.check_farm_claim_progress_energy(0);
}

#[test]
fn enter_farm_other_users_pos_test() {
    DebugApi::dummy();

    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    let second_user = farm_setup.second_user.clone();

    let first_user_energy_amount = 1_000;
    let second_user_energy_amount = 5_000;
    farm_setup.set_user_energy(&first_user, first_user_energy_amount, 13, 1);
    farm_setup.set_user_energy(&second_user, second_user_energy_amount, 13, 1);

    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    let token_attributes: FarmTokenAttributes<DebugApi> = farm_setup
        .b_mock
        .get_nft_attributes(&first_user, FARM_TOKEN_ID, 1)
        .unwrap();

    // first user transfer pos to second user
    farm_setup.b_mock.set_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(first_farm_token_amount),
        &token_attributes,
    );
    farm_setup.b_mock.set_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(0),
        &token_attributes,
    );

    let transfers = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(1_000),
        },
        TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(first_farm_token_amount),
        },
    ];

    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(&second_user, &farm_setup.farm_wrapper, &transfers, |sc| {
            sc.enter_farm_endpoint(OptionalValue::None);

            let actual_energy = sc.total_energy_for_week(1).get();
            assert_eq!(actual_energy, managed_biguint!(second_user_energy_amount));
        })
        .assert_ok();
}

#[test]
fn exit_other_users_pos_test() {
    DebugApi::dummy();

    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory_mock::contract_obj,
        energy_update::contract_obj,
    );

    let first_farm_token_amount = 100_000_000;
    let first_user = farm_setup.first_user.clone();
    let second_user = farm_setup.second_user.clone();

    let first_user_energy_amount = 1_000;
    let second_user_energy_amount = 200;
    farm_setup.set_user_energy(&first_user, first_user_energy_amount, 13, 1);
    farm_setup.set_user_energy(&second_user, second_user_energy_amount, 13, 1);

    farm_setup.enter_farm(&first_user, first_farm_token_amount);

    let token_attributes: FarmTokenAttributes<DebugApi> = farm_setup
        .b_mock
        .get_nft_attributes(&first_user, FARM_TOKEN_ID, 1)
        .unwrap();

    // first user transfer pos to second user
    farm_setup.b_mock.set_nft_balance(
        &second_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(first_farm_token_amount),
        &token_attributes,
    );
    farm_setup.b_mock.set_nft_balance(
        &first_user,
        FARM_TOKEN_ID,
        1,
        &rust_biguint!(0),
        &token_attributes,
    );

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &second_user,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(first_farm_token_amount),
            |sc| {
                sc.exit_farm_endpoint(OptionalValue::None);

                let actual_energy = sc.total_energy_for_week(1).get();
                assert_eq!(actual_energy, managed_biguint!(0));
            },
        )
        .assert_ok();
}
