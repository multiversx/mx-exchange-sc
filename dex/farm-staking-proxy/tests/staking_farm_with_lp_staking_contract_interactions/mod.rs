use elrond_wasm::types::{
    Address, BigUint, EsdtTokenPayment, ManagedAddress, ManagedVec, TokenIdentifier,
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, StateChange},
    DebugApi,
};

use ::config as farm_staking_config;
use farm_staking::whitelist::WhitelistModule;
use farm_staking::*;
use farm_staking_config::ConfigModule as _;

use farm_staking_proxy::dual_yield_token::DualYieldTokenModule;
use farm_staking_proxy::*;

use crate::{
    constants::*,
    staking_farm_with_lp_external_contracts::{setup_lp_farm, setup_pair},
    staking_farm_with_lp_staking_contract_setup::{
        add_proxy_to_whitelist, setup_proxy, setup_staking_farm,
    },
};

pub struct FarmStakingSetup<
    PairObjBuilder,
    FarmObjBuilder,
    StakingContractObjBuilder,
    ProxyContractObjBuilder,
> where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder: 'static + Copy + Fn() -> farm_staking_proxy::ContractObj<DebugApi>,
{
    pub owner_addr: Address,
    pub user_addr: Address,
    pub b_mock: BlockchainStateWrapper,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub lp_farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub staking_farm_wrapper:
        ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>,
    pub proxy_wrapper:
        ContractObjWrapper<farm_staking_proxy::ContractObj<DebugApi>, ProxyContractObjBuilder>,
}

impl<PairObjBuilder, FarmObjBuilder, StakingContractObjBuilder, ProxyContractObjBuilder>
    FarmStakingSetup<
        PairObjBuilder,
        FarmObjBuilder,
        StakingContractObjBuilder,
        ProxyContractObjBuilder,
    >
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder: 'static + Copy + Fn() -> farm_staking_proxy::ContractObj<DebugApi>,
{
    pub fn new(
        pair_builder: PairObjBuilder,
        lp_farm_builder: FarmObjBuilder,
        staking_farm_builder: StakingContractObjBuilder,
        proxy_builder: ProxyContractObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));

        let pair_wrapper = setup_pair(&owner_addr, &user_addr, &mut b_mock, pair_builder);
        let lp_farm_wrapper = setup_lp_farm(
            &owner_addr,
            &user_addr,
            &mut b_mock,
            lp_farm_builder,
            USER_TOTAL_LP_TOKENS,
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
            &staking_farm_wrapper,
        );

        FarmStakingSetup {
            owner_addr,
            user_addr,
            b_mock,
            pair_wrapper,
            lp_farm_wrapper,
            staking_farm_wrapper,
            proxy_wrapper,
        }
    }

    pub fn stake_farm_lp(&mut self, lp_farm_token_nonce: u64, lp_farm_token_stake_amount: u64) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.proxy_wrapper,
                LP_FARM_TOKEN_ID,
                lp_farm_token_nonce,
                &rust_biguint!(lp_farm_token_stake_amount),
                |sc| {
                    let payments = ManagedVec::from_single_item(EsdtTokenPayment::new(
                        managed_token_id!(LP_FARM_TOKEN_ID),
                        lp_farm_token_nonce,
                        managed_biguint!(lp_farm_token_stake_amount),
                    ));
                    sc.stake_farm_tokens(payments);

                    StateChange::Commit
                },
            )
            .assert_ok();
    }
}
