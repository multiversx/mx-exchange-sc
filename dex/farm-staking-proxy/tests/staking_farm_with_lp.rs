use elrond_wasm_debug::{
    rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

pub mod constants;
pub mod staking_farm_with_lp_external_contracts;
pub mod staking_farm_with_lp_staking_contract_interactions;

use constants::*;
use staking_farm_with_lp_external_contracts::*;
use staking_farm_with_lp_staking_contract_interactions::*;

struct FarmStakingSetup<
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
    pub b_mock: BlockchainStateWrapper,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub lp_farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub staking_farm_wrapper:
        ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>,
    pub proxy_wrapper:
        ContractObjWrapper<farm_staking_proxy::ContractObj<DebugApi>, ProxyContractObjBuilder>,
}

fn setup_all<PairObjBuilder, FarmObjBuilder, StakingContractObjBuilder, ProxyContractObjBuilder>(
    pair_builder: PairObjBuilder,
    lp_farm_builder: FarmObjBuilder,
    staking_farm_builder: StakingContractObjBuilder,
    proxy_builder: ProxyContractObjBuilder,
) -> FarmStakingSetup<
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
    let staking_farm_wrapper = setup_staking_farm(&owner_addr, &mut b_mock, staking_farm_builder);
    let proxy_wrapper = setup_proxy(
        &owner_addr,
        lp_farm_wrapper.address_ref(),
        staking_farm_wrapper.address_ref(),
        pair_wrapper.address_ref(),
        &mut b_mock,
        proxy_builder,
    );

    FarmStakingSetup {
        b_mock,
        pair_wrapper,
        lp_farm_wrapper,
        staking_farm_wrapper,
        proxy_wrapper,
    }
}

#[test]
fn test_all_setup() {
    let _ = setup_all(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );
}
