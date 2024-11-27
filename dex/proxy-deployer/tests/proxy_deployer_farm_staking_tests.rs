use farm_staking::custom_rewards::CustomRewardsModule;
use farm_token::FarmTokenModule;
use multiversx_sc::imports::StorageTokenWrapper;
use multiversx_sc_scenario::{managed_address, managed_biguint, managed_token_id, rust_biguint};
use proxy_deployer::{deploy::DeployModule, remove_contracts::RemoveContractsModule};
use proxy_deployer_farm_staking_setup::ProxyDeployerFarmStakingSetup;

pub mod proxy_deployer_farm_staking_setup;

#[test]
fn setup_test() {
    let _ = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );
}

#[test]
fn deploy_farm_staking_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_staking::contract_obj,
    );
    setup
        .b_mock
        .execute_tx(
            &setup.user,
            &setup.proxy_deployer_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.deploy_farm_staking_contract(
                    managed_token_id!(b"COOLTOK-123456"),
                    managed_biguint!(7_500),
                    10,
                );
            },
        )
        .assert_ok();

    // user call admin function on new farm staking
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
