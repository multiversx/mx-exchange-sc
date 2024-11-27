use common_structs::Timestamp;
use multiversx_sc::types::Address;
use multiversx_sc_scenario::{
    imports::{BlockchainStateWrapper, ContractObjWrapper},
    managed_address, rust_biguint, DebugApi,
};
use proxy_deployer::{storage::DeployerType, ProxyDeployer};
use timestamp_oracle::{epoch_to_timestamp::EpochToTimestampModule, TimestampOracle};

pub const TIMESTAMP_PER_EPOCH: Timestamp = 24 * 60 * 60;

pub struct ProxyDeployerFarmStakingSetup<ProxyDeployerBuilder, FarmStakingBuilder>
where
    ProxyDeployerBuilder: 'static + Copy + Fn() -> proxy_deployer::ContractObj<DebugApi>,
    FarmStakingBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub user: Address,
    pub proxy_deployer_wrapper:
        ContractObjWrapper<proxy_deployer::ContractObj<DebugApi>, ProxyDeployerBuilder>,
    pub template_wrapper:
        ContractObjWrapper<farm_staking::ContractObj<DebugApi>, FarmStakingBuilder>,
}

impl<ProxyDeployerBuilder, FarmStakingBuilder>
    ProxyDeployerFarmStakingSetup<ProxyDeployerBuilder, FarmStakingBuilder>
where
    ProxyDeployerBuilder: 'static + Copy + Fn() -> proxy_deployer::ContractObj<DebugApi>,
    FarmStakingBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    pub fn new(
        proxy_builder: ProxyDeployerBuilder,
        farm_staking_builder: FarmStakingBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let user = b_mock.create_user_account(&rust_zero);
        let proxy_deployer_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), proxy_builder, "proxy deployer");
        let template_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            farm_staking_builder,
            "farm staking template",
        );

        let timestamp_oracle_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            timestamp_oracle::contract_obj,
            "timestamp oracle",
        );
        b_mock
            .execute_tx(&owner, &timestamp_oracle_wrapper, &rust_zero, |sc| {
                sc.init(0);

                for i in 0..=21 {
                    sc.set_start_timestamp_for_epoch(i, i * TIMESTAMP_PER_EPOCH + 1);
                }
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &proxy_deployer_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_address!(template_wrapper.address_ref()),
                    DeployerType::FarmStaking,
                    managed_address!(timestamp_oracle_wrapper.address_ref()),
                );
            })
            .assert_ok();

        Self {
            b_mock,
            owner,
            user,
            proxy_deployer_wrapper,
            template_wrapper,
        }
    }
}
