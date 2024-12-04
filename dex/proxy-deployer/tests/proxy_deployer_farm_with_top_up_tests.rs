use farm_token::FarmTokenModule;
use farm_with_top_up::{custom_rewards::CustomRewardsModule, FarmWithTopUp};
use multiversx_sc::{
    codec::Empty,
    imports::{OptionalValue, StorageTokenWrapper},
    types::EsdtLocalRole,
};
use multiversx_sc_scenario::{managed_address, managed_biguint, managed_token_id, rust_biguint};
use proxy_deployer::remove_contracts::RemoveContractsModule;
use proxy_deployer_farm_with_top_up_setup::ProxyDeployerFarmStakingSetup;

pub mod proxy_deployer_farm_with_top_up_setup;

#[test]
fn setup_test() {
    let _ = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_with_top_up::contract_obj,
    );
}

#[test]
fn deploy_farm_staking_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_with_top_up::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_with_top_up::contract_obj,
    );
    setup.deploy_farm_with_top_up(&b"COOLTOK-123456"[..], &b"COOLERTOK-123456"[..]);

    // user call admin function on new farm
    setup
        .b_mock
        .execute_tx(&setup.user, &new_sc_wrapper, &rust_biguint!(0), |sc| {
            sc.farm_token()
                .set_token_id(managed_token_id!(b"MYCOOLFARM-123456"));

            sc.set_per_block_rewards(managed_biguint!(1_000));
        })
        .assert_ok();

    // owner remove the contracts
    let user_addr = setup.user.clone();
    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.proxy_deployer_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.remove_all_by_deployer(managed_address!(&user_addr), 1);
            },
        )
        .assert_ok();

    // user try call admin function after removed
    setup
        .b_mock
        .execute_tx(&setup.user, &new_sc_wrapper, &rust_biguint!(0), |sc| {
            sc.set_per_block_rewards(managed_biguint!(1_000));
        })
        .assert_user_error("Permission denied");
}

#[test]
fn set_contract_active_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_with_top_up::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_with_top_up::contract_obj,
    );
    let farming_token_id = b"COOLTOK-123456";
    let farm_token_id = b"MYCOOLFARM-123456";
    setup.deploy_farm_with_top_up(&farming_token_id[..], &farming_token_id[..]);

    // simulate farm token issue
    setup
        .b_mock
        .execute_tx(&setup.user, &new_sc_wrapper, &rust_biguint!(0), |sc| {
            sc.farm_token()
                .set_token_id(managed_token_id!(farm_token_id));
        })
        .assert_ok();

    setup.b_mock.set_esdt_local_roles(
        new_sc_wrapper.address_ref(),
        farm_token_id,
        &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftBurn],
    );

    // set user balance
    setup
        .b_mock
        .set_esdt_balance(&setup.user, &farming_token_id[..], &rust_biguint!(1_000));

    // user try enter farm before it's ready
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user,
            &new_sc_wrapper,
            &farming_token_id[..],
            0,
            &rust_biguint!(1_000),
            |sc| {
                sc.enter_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_user_error("Not active");

    setup.set_contract_active(new_sc_wrapper.address_ref());

    // user enter farm again
    setup
        .b_mock
        .execute_esdt_transfer(
            &setup.user,
            &new_sc_wrapper,
            &farming_token_id[..],
            0,
            &rust_biguint!(1_000),
            |sc| {
                sc.enter_farm_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &setup.user,
        &farm_token_id[..],
        1,
        &rust_biguint!(1_000),
        Option::<&Empty>::None,
    );
}
