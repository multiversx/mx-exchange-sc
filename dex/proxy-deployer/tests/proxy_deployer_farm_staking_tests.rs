use proxy_deployer_farm_staking_setup::ProxyDeployerFarmStakingSetup;

pub mod proxy_deployer_farm_staking_setup;

#[test]
fn setup_test() {
    let _ = ProxyDeployerFarmStakingSetup::new(
        proxy_deployer::contract_obj,
        farm_staking::contract_obj,
    );
}
