use elrond_wasm::types::{Address, BigUint, ManagedAddress, SCResult, TokenIdentifier};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, StateChange},
    DebugApi,
};

use ::config as farm_staking_config;
use farm_staking::*;
use farm_staking_config::ConfigModule as _;

use farm_staking::whitelist::WhitelistModule;

const STAKING_FARM_WASM_PATH: &str = "farm-staking/output/farm-staking.wasm";
const STAKING_REWARD_TOKEN_ID: &[u8] = b"RIDE-abcdef";
const STAKING_TOKEN_ID: &[u8] = STAKING_REWARD_TOKEN_ID;
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const MAX_APR: u64 = 5_000; // 50%
const UNBOND_EPOCHS: u64 = 10;

pub fn setup_staking_farm<StakingContractObjBuilder>(
    owner_addr: &Address,
    pair_address: &Address,
    blockchain_wrapper: &mut BlockchainStateWrapper,
    builder: StakingContractObjBuilder,
) -> ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>
where
    StakingContractObjBuilder: 'static + Copy + Fn(DebugApi) -> farm_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let farm_staking_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(owner_addr),
        builder,
        STAKING_FARM_WASM_PATH,
    );

    blockchain_wrapper.execute_tx(&owner_addr, &farm_staking_wrapper, &rust_zero, |sc| {
        let reward_token_id = managed_token_id!(STAKING_REWARD_TOKEN_ID);
        let farming_token_id = managed_token_id!(STAKING_TOKEN_ID);
        let div_const = managed_biguint!(DIVISION_SAFETY_CONSTANT);
        let pair_addr = managed_address!(pair_address);
        let max_apr = managed_biguint!(MAX_APR);

        let result = sc.init(
            reward_token_id,
            farming_token_id,
            div_const,
            pair_addr,
            max_apr,
            UNBOND_EPOCHS,
        );
        assert_eq!(result, SCResult::Ok(()));

        sc.state().set(&farm_staking_config::State::Active);

        StateChange::Commit
    });

    farm_staking_wrapper
}

pub fn add_proxy_to_whitelist<StakingContractObjBuilder>(
    owner_addr: &Address,
    proxy_address: &Address,
    blockchain_wrapper: &mut BlockchainStateWrapper,
    staking_farm_builder: &ContractObjWrapper<
        farm_staking::ContractObj<DebugApi>,
        StakingContractObjBuilder,
    >,
) where
    StakingContractObjBuilder: 'static + Copy + Fn(DebugApi) -> farm_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    blockchain_wrapper.execute_tx(owner_addr, staking_farm_builder, &rust_zero, |sc| {
        sc.add_address_to_whitelist(managed_address!(proxy_address));

        StateChange::Commit
    })
}
