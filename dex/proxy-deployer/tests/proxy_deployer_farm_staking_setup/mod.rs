use common_structs::{Epoch, Timestamp};
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

    pub fn deploy_farm_staking(&mut self, token_id: &[u8], max_apr: u64, min_unbond_epochs: Epoch) {
        self.b_mock
            .execute_tx(
                &self.user,
                &self.proxy_deployer_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.deploy_farm_staking_contract(
                        managed_token_id!(token_id),
                        managed_biguint!(max_apr),
                        min_unbond_epochs,
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
