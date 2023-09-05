#![allow(deprecated)]

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use ::config as farm_staking_config;
use farm_staking::*;
use farm_staking_config::ConfigModule as _;

use farm_staking::custom_rewards::CustomRewardsModule;
use farm_staking_proxy::dual_yield_token::DualYieldTokenModule;

use farm_staking_proxy::*;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};
use sc_whitelist_module::SCWhitelistModule;

use crate::constants::*;

pub fn setup_staking_farm<StakingContractObjBuilder>(
    owner_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    builder: StakingContractObjBuilder,
) -> ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>
where
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let farm_staking_wrapper =
        b_mock.create_sc_account(&rust_zero, Some(owner_addr), builder, PROXY_WASM_PATH);

    b_mock
        .execute_tx(owner_addr, &farm_staking_wrapper, &rust_zero, |sc| {
            let farming_token_id = managed_token_id!(STAKING_TOKEN_ID);
            let div_const = managed_biguint!(DIVISION_SAFETY_CONSTANT);
            let max_apr = managed_biguint!(MAX_APR);

            sc.init(
                farming_token_id,
                div_const,
                max_apr,
                UNBOND_EPOCHS,
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::new(),
            );

            sc.farm_token()
                .set_token_id(managed_token_id!(STAKING_FARM_TOKEN_ID));

            sc.state().set(State::Active);
            sc.produce_rewards_enabled().set(true);
            sc.per_block_reward_amount()
                .set(&managed_biguint!(STAKING_FARM_PER_BLOCK_REWARD_AMOUNT));
            sc.last_reward_block_nonce()
                .set(BLOCK_NONCE_AFTER_PAIR_SETUP);
            sc.reward_capacity().set(&managed_biguint!(REWARD_CAPACITY));
        })
        .assert_ok();

    b_mock.set_esdt_balance(
        farm_staking_wrapper.address_ref(),
        STAKING_REWARD_TOKEN_ID,
        &rust_biguint!(REWARD_CAPACITY),
    );

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    b_mock.set_esdt_local_roles(
        farm_staking_wrapper.address_ref(),
        STAKING_FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    farm_staking_wrapper
}

pub fn add_proxy_to_whitelist<StakingContractObjBuilder>(
    owner_addr: &Address,
    proxy_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    staking_farm_builder: &ContractObjWrapper<
        farm_staking::ContractObj<DebugApi>,
        StakingContractObjBuilder,
    >,
) where
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    b_mock
        .execute_tx(owner_addr, staking_farm_builder, &rust_zero, |sc| {
            sc.add_sc_address_to_whitelist(managed_address!(proxy_address));
        })
        .assert_ok();
}

pub fn setup_proxy<ProxyContractObjBuilder>(
    owner_addr: &Address,
    lp_farm_address: &Address,
    staking_farm_address: &Address,
    pair_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    builder: ProxyContractObjBuilder,
) -> ContractObjWrapper<farm_staking_proxy::ContractObj<DebugApi>, ProxyContractObjBuilder>
where
    ProxyContractObjBuilder: 'static + Copy + Fn() -> farm_staking_proxy::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let proxy_wrapper =
        b_mock.create_sc_account(&rust_zero, Some(owner_addr), builder, PROXY_WASM_PATH);

    b_mock
        .execute_tx(owner_addr, &proxy_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_address!(lp_farm_address),
                managed_address!(staking_farm_address),
                managed_address!(pair_address),
                managed_token_id!(STAKING_TOKEN_ID),
                managed_token_id!(LP_FARM_TOKEN_ID),
                managed_token_id!(STAKING_FARM_TOKEN_ID),
                managed_token_id!(LP_TOKEN_ID),
            );

            sc.dual_yield_token()
                .set_token_id(managed_token_id!(DUAL_YIELD_TOKEN_ID));
        })
        .assert_ok();

    let dual_yield_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    b_mock.set_esdt_local_roles(
        proxy_wrapper.address_ref(),
        DUAL_YIELD_TOKEN_ID,
        &dual_yield_token_roles[..],
    );

    proxy_wrapper
}
