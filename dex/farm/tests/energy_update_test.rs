#![allow(deprecated)]

mod farm_setup;

use farm_setup::multi_user_farm_setup::*;

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
