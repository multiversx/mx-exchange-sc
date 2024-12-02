use farm_staking::{custom_rewards::CustomRewardsModule, stake_farm::StakeFarmModule};
use farm_token::FarmTokenModule;
use multiversx_sc::{
    codec::Empty,
    imports::{OptionalValue, StorageTokenWrapper},
    types::EsdtLocalRole,
};
use multiversx_sc_scenario::{managed_address, managed_biguint, managed_token_id, rust_biguint};
use proxy_deployer::{remove_contracts::RemoveContractsModule, views::ViewModule};
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
    setup.deploy_farm_staking(&b"COOLTOK-123456"[..], 7_500, 10);

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

#[test]
fn remove_single_contract_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_staking::contract_obj,
    );
    setup.deploy_farm_staking(&b"COOLTOK-123456"[..], 7_500, 10);

    // owner remove the contract
    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.proxy_deployer_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.remove_single_contract(managed_address!(new_sc_wrapper.address_ref()));
            },
        )
        .assert_ok();
}

#[test]
fn user_remove_contract_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_staking::contract_obj,
    );

    setup.deploy_farm_staking(&b"COOLTOK-123456"[..], 7_500, 10);

    // user remove the contract
    setup
        .b_mock
        .execute_tx(
            &setup.user,
            &setup.proxy_deployer_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.remove_own_contract(managed_address!(new_sc_wrapper.address_ref()));
            },
        )
        .assert_ok();
}

#[test]
fn set_contract_active_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_staking::contract_obj,
    );
    let farming_token_id = b"COOLTOK-123456";
    let farm_token_id = b"MYCOOLFARM-123456";
    setup.deploy_farm_staking(&farming_token_id[..], 7_500, 10);

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
                sc.stake_farm_endpoint(OptionalValue::None);
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
                sc.stake_farm_endpoint(OptionalValue::None);
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

#[test]
fn views_test() {
    let mut setup = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );

    let new_sc_wrapper = setup.b_mock.prepare_deploy_from_sc(
        setup.proxy_deployer_wrapper.address_ref(),
        farm_staking::contract_obj,
    );
    let farming_token_id = b"COOLTOK-123456";
    setup.deploy_farm_staking(&farming_token_id[..], 7_500, 10);

    let user_addr = setup.user.clone();
    setup
        .b_mock
        .execute_query(&setup.proxy_deployer_wrapper, |sc| {
            let is_blacklisted = sc.is_user_blacklisted(managed_address!(&user_addr));
            assert!(!is_blacklisted);

            let addr_for_tok = sc
                .get_address_for_token(managed_token_id!(farming_token_id))
                .into_option()
                .unwrap();
            assert_eq!(addr_for_tok, managed_address!(new_sc_wrapper.address_ref()));

            let token_for_addr = sc
                .get_token_for_address(managed_address!(new_sc_wrapper.address_ref()))
                .into_option()
                .unwrap();
            assert_eq!(token_for_addr, managed_token_id!(farming_token_id));

            let contract_owner = sc
                .get_contract_owner(managed_address!(new_sc_wrapper.address_ref()))
                .into_option()
                .unwrap();
            assert_eq!(contract_owner, managed_address!(&user_addr));

            let all_used_tokens = sc.get_all_used_tokens(1, 1_000);
            assert_eq!(all_used_tokens.len(), 1);
            assert_eq!(
                (*all_used_tokens.to_vec().get(0)).clone(),
                managed_token_id!(farming_token_id)
            );

            let all_deployed_contracts = sc.get_all_deployed_contracts_by_sc(1, 1_000);
            assert_eq!(all_deployed_contracts.len(), 1);
            assert_eq!(
                (*all_deployed_contracts.to_vec().get(0)).clone(),
                managed_address!(new_sc_wrapper.address_ref())
            );

            let all_deployed_by_user =
                sc.get_all_deployed_contracts_by_user(managed_address!(&user_addr), 1, 1_000);
            assert_eq!(all_deployed_by_user.len(), 1);
            assert_eq!(
                (*all_deployed_by_user.to_vec().get(0)).clone(),
                managed_address!(new_sc_wrapper.address_ref())
            );
        })
        .assert_ok();
}
