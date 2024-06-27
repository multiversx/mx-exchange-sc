use multiversx_sc::types::Address;
use multiversx_sc_scenario::{
    managed_biguint, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use farm_staking::*;
use farm_staking_proxy_legacy::*;
use token_attributes::UnbondSftAttributes;
use unbond_farm::UnbondFarmModule;

use crate::{
    constants::*,
    staking_farm_with_lp_external_contracts::{setup_energy_factory, setup_lp_farm, setup_pair},
    staking_farm_with_lp_staking_contract_setup::{
        add_proxy_to_whitelist, setup_proxy, setup_staking_farm,
    },
};

pub struct FarmStakingSetup<
    PairObjBuilder,
    FarmObjBuilder,
    EnergyFactoryObjBuilder,
    StakingContractObjBuilder,
    ProxyContractObjBuilder,
> where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder:
        'static + Copy + Fn() -> farm_staking_proxy_legacy::ContractObj<DebugApi>,
{
    pub owner_addr: Address,
    pub user_addr: Address,
    pub b_mock: BlockchainStateWrapper,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub lp_farm_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryObjBuilder>,
    pub staking_farm_wrapper:
        ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>,
    pub proxy_wrapper: ContractObjWrapper<
        farm_staking_proxy_legacy::ContractObj<DebugApi>,
        ProxyContractObjBuilder,
    >,
}

impl<
        PairObjBuilder,
        FarmObjBuilder,
        EnergyFactoryObjBuilder,
        StakingContractObjBuilder,
        ProxyContractObjBuilder,
    >
    FarmStakingSetup<
        PairObjBuilder,
        FarmObjBuilder,
        EnergyFactoryObjBuilder,
        StakingContractObjBuilder,
        ProxyContractObjBuilder,
    >
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder:
        'static + Copy + Fn() -> farm_staking_proxy_legacy::ContractObj<DebugApi>,
{
    pub fn new(
        pair_builder: PairObjBuilder,
        lp_farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryObjBuilder,
        staking_farm_builder: StakingContractObjBuilder,
        proxy_builder: ProxyContractObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));

        let pair_wrapper = setup_pair(&owner_addr, &user_addr, &mut b_mock, pair_builder);
        let energy_factory_wrapper =
            setup_energy_factory(&owner_addr, &mut b_mock, energy_factory_builder);
        let lp_farm_wrapper = setup_lp_farm(
            &owner_addr,
            energy_factory_wrapper.address_ref(),
            &mut b_mock,
            lp_farm_builder,
        );

        let staking_farm_wrapper =
            setup_staking_farm(&owner_addr, &mut b_mock, staking_farm_builder);
        let proxy_wrapper = setup_proxy(
            &owner_addr,
            lp_farm_wrapper.address_ref(),
            staking_farm_wrapper.address_ref(),
            pair_wrapper.address_ref(),
            &mut b_mock,
            proxy_builder,
        );

        add_proxy_to_whitelist(
            &owner_addr,
            proxy_wrapper.address_ref(),
            &mut b_mock,
            &lp_farm_wrapper,
            &staking_farm_wrapper,
        );

        FarmStakingSetup {
            owner_addr,
            user_addr,
            b_mock,
            pair_wrapper,
            lp_farm_wrapper,
            energy_factory_wrapper,
            staking_farm_wrapper,
            proxy_wrapper,
        }
    }

    pub fn unstake_proxy(
        &mut self,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
        expected_wegld_amount: u64,
        expected_unbond_token_amount: u64,
        expected_unbond_token_unlock_epoch: u64,
    ) -> u64 {
        let mut unbond_token_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.proxy_wrapper,
                DUAL_YIELD_TOKEN_ID,
                dual_yield_token_nonce,
                &rust_biguint!(dual_yield_token_amount),
                |sc| {
                    let received_tokens = sc
                        .unstake_farm_tokens(managed_biguint!(1), managed_biguint!(1))
                        .to_vec();
                    let mut vec_index = 0;

                    if expected_wegld_amount > 0 {
                        let wegld_payment = received_tokens.get(vec_index);
                        assert_eq!(wegld_payment.amount, expected_wegld_amount);

                        vec_index += 1;
                    }

                    let unbond_tokens = received_tokens.get(vec_index);
                    assert_eq!(unbond_tokens.amount, expected_unbond_token_amount);

                    unbond_token_nonce = unbond_tokens.token_nonce;
                },
            )
            .assert_ok();

        self.b_mock.execute_in_managed_environment(|| {
            let expected_attributes = UnbondSftAttributes {
                unlock_epoch: expected_unbond_token_unlock_epoch,
            };

            self.b_mock.check_nft_balance(
                &self.user_addr,
                STAKING_FARM_TOKEN_ID,
                unbond_token_nonce,
                &rust_biguint!(expected_unbond_token_amount),
                Some(&expected_attributes),
            );
        });

        unbond_token_nonce
    }

    pub fn unbond_proxy(
        &mut self,
        unbond_token_nonce: u64,
        unbond_token_amount: u64,
        expected_token_out_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.staking_farm_wrapper,
                STAKING_FARM_TOKEN_ID,
                unbond_token_nonce,
                &rust_biguint!(unbond_token_amount),
                |sc| {
                    let received_tokens = sc.unbond_farm();
                    assert_eq!(received_tokens.amount, expected_token_out_amount);
                },
            )
            .assert_ok();
    }
}
