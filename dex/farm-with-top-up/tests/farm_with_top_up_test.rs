use farm_with_top_up_setup::FarmWithTopUpSetup;

pub mod farm_with_top_up_setup;

#[test]
fn setup_farm_with_top_up_test() {
    let _ = FarmWithTopUpSetup::new(
        farm_with_top_up::contract_obj,
        timestamp_oracle::contract_obj,
        permissions_hub::contract_obj,
    );
}
