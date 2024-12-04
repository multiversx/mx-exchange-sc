use common_structs::Timestamp;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactors;
use multiversx_sc::types::Address;
use multiversx_sc_scenario::{
    imports::{BlockchainStateWrapper, ContractObjWrapper},
    managed_address, managed_biguint, managed_token_id, rust_biguint, DebugApi,
};
use proxy_deployer::{
    deploy::DeployModule, set_contract_active::SetContractActiveModule, storage::DeployerType,
    ProxyDeployer,
};
use timestamp_oracle::{epoch_to_timestamp::EpochToTimestampModule, TimestampOracle};

pub const TIMESTAMP_PER_EPOCH: Timestamp = 24 * 60 * 60;

pub struct ProxyDeployerFarmStakingSetup<ProxyDeployerBuilder, FarmWithTopUpBuilder>
where
    ProxyDeployerBuilder: 'static + Copy + Fn() -> proxy_deployer::ContractObj<DebugApi>,
    FarmWithTopUpBuilder: 'static + Copy + Fn() -> farm_with_top_up::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub user: Address,
    pub proxy_deployer_wrapper:
        ContractObjWrapper<proxy_deployer::ContractObj<DebugApi>, ProxyDeployerBuilder>,
    pub template_wrapper:
        ContractObjWrapper<farm_with_top_up::ContractObj<DebugApi>, FarmWithTopUpBuilder>,
}

impl<ProxyDeployerBuilder, FarmWithTopUpBuilder>
    ProxyDeployerFarmStakingSetup<ProxyDeployerBuilder, FarmWithTopUpBuilder>
where
    ProxyDeployerBuilder: 'static + Copy + Fn() -> proxy_deployer::ContractObj<DebugApi>,
    FarmWithTopUpBuilder: 'static + Copy + Fn() -> farm_with_top_up::ContractObj<DebugApi>,
{
    pub fn new(
        proxy_builder: ProxyDeployerBuilder,
        farm_with_top_up_builder: FarmWithTopUpBuilder,
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
            farm_with_top_up_builder,
            "farm top up template",
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
                    DeployerType::FarmWithTopUp,
                    managed_address!(timestamp_oracle_wrapper.address_ref()),
                    BoostedYieldsFactors {
                        max_rewards_factor: managed_biguint!(10),
                        user_rewards_energy_const: managed_biguint!(3),
                        user_rewards_farm_const: managed_biguint!(2),
                        min_energy_amount: managed_biguint!(1),
                        min_farm_amount: managed_biguint!(1),
                    },
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

    pub fn deploy_farm_with_top_up(&mut self, farming_token_id: &[u8], reward_token_id: &[u8]) {
        self.b_mock
            .execute_tx(
                &self.user,
                &self.proxy_deployer_wrapper,
                &rust_biguint!(0),
                |sc| {
                    let _ = sc.deploy_farm_with_top_up(
                        managed_token_id!(farming_token_id),
                        managed_token_id!(reward_token_id),
                    );
                },
            )
            .assert_ok();
    }

    pub fn set_contract_active(&mut self, contract: &Address) {
        self.b_mock
            .execute_tx(
                &self.user,
                &self.proxy_deployer_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_contract_active(
                        managed_address!(contract),
                        managed_biguint!(1_000),
                        1_000, // 10%
                    );
                },
            )
            .assert_ok();
    }
}
