use energy_factory::token_whitelist::TokenWhitelistModule;
use energy_factory::SimpleLockEnergy;
use energy_query::EnergyQueryModule;
use farm_token::FarmTokenModule;
use farm_with_locked_rewards::Farm;
use multiversx_sc::imports::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, MultiValueEncoded};
use multiversx_sc_modules::pause::PauseModule;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};

use pair::config as pair_config;
use pair::*;
use pair_actions::add_liq::AddLiquidityModule;
use pair_actions::initial_liq::InitialLiquidityModule;
use pair_config::ConfigModule as _;

use pausable::{PausableModule, State};
use simple_lock::locked_token::LockedTokenModule as _;

use crate::constants::*;

pub fn setup_pair<PairObjBuilder>(
    owner_addr: &Address,
    user_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    pair_builder: PairObjBuilder,
) -> ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let pair_wrapper =
        b_mock.create_sc_account(&rust_zero, Some(owner_addr), pair_builder, PAIR_WASM_PATH);

    b_mock
        .execute_tx(owner_addr, &pair_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
            let second_token_id = managed_token_id!(RIDE_TOKEN_ID);
            let router_address = managed_address!(owner_addr);
            let router_owner_address = managed_address!(owner_addr);
            let initial_liquidity_adder = managed_address!(owner_addr);
            let total_fee_percent = 300u64;
            let special_fee_percent = 50u64;

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                total_fee_percent,
                special_fee_percent,
                initial_liquidity_adder,
                MultiValueEncoded::new(),
            );

            let lp_token_id = managed_token_id!(LP_TOKEN_ID);
            sc.lp_token_identifier().set(&lp_token_id);
        })
        .assert_ok();

    let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
    b_mock.set_esdt_local_roles(pair_wrapper.address_ref(), LP_TOKEN_ID, &lp_token_roles[..]);

    // set user balance
    b_mock.set_esdt_balance(
        user_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
    );
    b_mock.set_esdt_balance(
        user_addr,
        RIDE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    b_mock.set_block_nonce(BLOCK_NONCE_FIRST_ADD_LIQ);

    b_mock.set_esdt_balance(
        owner_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
    );
    b_mock.set_esdt_balance(
        owner_addr,
        RIDE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    let payments = vec![
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(1_000_000_000),
        },
        TxTokenTransfer {
            token_identifier: RIDE_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(1_000_000_000),
        },
    ];
    b_mock
        .execute_esdt_multi_transfer(owner_addr, &pair_wrapper, &payments, |sc| {
            sc.add_initial_liquidity();
        })
        .assert_ok();

    b_mock.set_block_nonce(BLOCK_NONCE_SECOND_ADD_LIQ);

    b_mock
        .execute_esdt_multi_transfer(owner_addr, &pair_wrapper, &payments, |sc| {
            sc.add_liquidity(managed_biguint!(1u64), managed_biguint!(1u64));
        })
        .assert_ok();

    b_mock.set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP);

    pair_wrapper
}

pub fn setup_lp_farm<FarmObjBuilder>(
    owner_addr: &Address,
    energy_factory_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    farm_builder: FarmObjBuilder,
) -> ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let farm_wrapper =
        b_mock.create_sc_account(&rust_zero, Some(owner_addr), farm_builder, FARM_WASM_PATH);

    // init farm contract

    b_mock
        .execute_tx(owner_addr, &farm_wrapper, &rust_zero, |sc| {
            let reward_token_id = managed_token_id!(RIDE_TOKEN_ID);
            let farming_token_id = managed_token_id!(LP_TOKEN_ID);
            let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
            let pair_address = managed_address!(&Address::zero());

            sc.init(
                reward_token_id,
                farming_token_id,
                division_safety_constant,
                pair_address,
                managed_address!(owner_addr),
                MultiValueEncoded::new(),
            );

            let farm_token_id = managed_token_id!(LP_FARM_TOKEN_ID);
            sc.farm_token().set_token_id(farm_token_id);

            sc.energy_factory_address()
                .set(managed_address!(energy_factory_address));

            sc.state().set(State::Active);
        })
        .assert_ok();

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        LP_FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    let farming_token_roles = [EsdtLocalRole::Burn];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        LP_TOKEN_ID,
        &farming_token_roles[..],
    );

    let reward_token_roles = [EsdtLocalRole::Mint];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        RIDE_TOKEN_ID,
        &reward_token_roles[..],
    );

    farm_wrapper
}

pub fn setup_energy_factory<EnergyFactoryObjBuilder>(
    owner_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    energy_factory_builder: EnergyFactoryObjBuilder,
) -> ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryObjBuilder>
where
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let energy_factory_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(owner_addr),
        energy_factory_builder,
        "energy factory",
    );

    b_mock
        .execute_tx(owner_addr, &energy_factory_wrapper, &rust_zero, |sc| {
            let mut lock_options = MultiValueEncoded::new();
            for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                lock_options.push((*option, *penalty).into());
            }

            sc.init(
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                managed_address!(energy_factory_wrapper.address_ref()),
                0,
                lock_options,
            );

            sc.base_asset_token_id()
                .set(managed_token_id!(MEX_TOKEN_ID));
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.set_paused(false);
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        energy_factory_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );
    b_mock.set_esdt_local_roles(
        energy_factory_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ],
    );
    b_mock.set_esdt_local_roles(
        energy_factory_wrapper.address_ref(),
        LEGACY_LOCKED_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    energy_factory_wrapper
}
