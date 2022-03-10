use elrond_wasm::elrond_codec::multi_types::{MultiValue3, OptionalValue};
use elrond_wasm::types::{Address, EsdtLocalRole};
use elrond_wasm_debug::tx_mock::TxInputESDT;
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};

const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";
const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

const LKWEGLD_TOKEN_ID: &[u8] = b"LKWEGLD-abcdef";

const USER_TOTAL_MEX_TOKENS: u64 = 5_000_000_000;
const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;

use pair::bot_protection::*;
use pair::config::*;
use pair::errors::*;
use pair::locked_asset::*;
use pair::safe_price::*;
use pair::*;

#[allow(dead_code)]
struct PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

fn setup_pair<PairObjBuilder>(pair_builder: PairObjBuilder) -> PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
    let pair_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        pair_builder,
        PAIR_WASM_PATH,
    );

    blockchain_wrapper
        .execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
            let second_token_id = managed_token_id!(MEX_TOKEN_ID);
            let router_address = managed_address!(&owner_addr);
            let router_owner_address = managed_address!(&owner_addr);
            let total_fee_percent = 300u64;
            let special_fee_percent = 50u64;

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                total_fee_percent,
                special_fee_percent,
                OptionalValue::None,
            );

            let lp_token_id = managed_token_id!(LP_TOKEN_ID);
            sc.lp_token_identifier().set(&lp_token_id);

            sc.state().set(&State::Active);
            sc.set_max_observations_per_record(10);
        })
        .assert_ok();

    let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
    blockchain_wrapper.set_esdt_local_roles(
        pair_wrapper.address_ref(),
        LP_TOKEN_ID,
        &lp_token_roles[..],
    );

    let user_addr = blockchain_wrapper.create_user_account(&rust_biguint!(100_000_000));
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        MEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );

    PairSetup {
        blockchain_wrapper,
        owner_address: owner_addr,
        user_address: user_addr,
        pair_wrapper,
    }
}

fn add_liquidity<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    first_token_amount: u64,
    first_token_min: u64,
    second_token_amount: u64,
    second_token_min: u64,
    expected_lp_amount: u64,
    expected_first_amount: u64,
    expected_second_amount: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    let payments = vec![
        TxInputESDT {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(first_token_amount),
        },
        TxInputESDT {
            token_identifier: MEX_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(second_token_amount),
        },
    ];

    pair_setup
        .blockchain_wrapper
        .execute_esdt_multi_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &payments,
            |sc| {
                let MultiValue3 { 0: payments } = sc.add_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                    OptionalValue::None,
                );

                assert_eq!(payments.0.token_identifier, managed_token_id!(LP_TOKEN_ID));
                assert_eq!(payments.0.token_nonce, 0);
                assert_eq!(payments.0.amount, managed_biguint!(expected_lp_amount));

                assert_eq!(
                    payments.1.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(payments.1.token_nonce, 0);
                assert_eq!(payments.1.amount, managed_biguint!(expected_first_amount));

                assert_eq!(payments.2.token_identifier, managed_token_id!(MEX_TOKEN_ID));
                assert_eq!(payments.2.token_nonce, 0);
                assert_eq!(payments.2.amount, managed_biguint!(expected_second_amount));
            },
        )
        .assert_ok();
}

fn swap_fixed_input<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    payment_token_id: &[u8],
    payment_amount: u64,
    desired_token_id: &[u8],
    desired_amount_min: u64,
    expected_amount: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pair_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &payment_token_id,
            0,
            &rust_biguint!(payment_amount),
            |sc| {
                let ret = sc.swap_tokens_fixed_input(
                    managed_token_id!(payment_token_id),
                    0,
                    managed_biguint!(payment_amount),
                    managed_token_id!(desired_token_id),
                    managed_biguint!(desired_amount_min),
                    OptionalValue::None,
                );

                assert_eq!(ret.token_identifier, managed_token_id!(desired_token_id));
                assert_eq!(ret.token_nonce, 0);
                assert_eq!(ret.amount, managed_biguint!(expected_amount));
            },
        )
        .assert_ok();
}

fn swap_fixed_input_expect_error<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    payment_token_id: &[u8],
    payment_amount: u64,
    desired_token_id: &[u8],
    desired_amount_min: u64,
    expected_message: &str,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pair_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &payment_token_id,
            0,
            &rust_biguint!(payment_amount),
            |sc| {
                sc.swap_tokens_fixed_input(
                    managed_token_id!(payment_token_id),
                    0,
                    managed_biguint!(payment_amount),
                    managed_token_id!(desired_token_id),
                    managed_biguint!(desired_amount_min),
                    OptionalValue::None,
                );
            },
        )
        .assert_user_error(expected_message);
}

fn set_swap_protect<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    protect_stop_block: u64,
    volume_percent: u64,
    max_num_actions_per_address: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pair_setup
        .blockchain_wrapper
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_bp_swap_config(
                    protect_stop_block,
                    volume_percent,
                    max_num_actions_per_address,
                );
            },
        )
        .assert_ok();
}

fn check_current_safe_state<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    from: u64,
    to: u64,
    num_obs: u64,
    first_reserve_last_obs: u64,
    second_reserve_last_obs: u64,
    first_reserve_weighted: u64,
    second_reserve_weighted: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pair_setup
        .blockchain_wrapper
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let state = sc.get_current_state_or_default();

            assert_eq!(state.first_obs_block, from);
            assert_eq!(state.last_obs_block, to);
            assert_eq!(state.num_observations, num_obs);
            assert_eq!(
                state.first_token_reserve_last_obs,
                managed_biguint!(first_reserve_last_obs)
            );
            assert_eq!(
                state.second_token_reserve_last_obs,
                managed_biguint!(second_reserve_last_obs)
            );
            assert_eq!(
                state.first_token_reserve_weighted,
                managed_biguint!(first_reserve_weighted)
            );
            assert_eq!(
                state.second_token_reserve_weighted,
                managed_biguint!(second_reserve_weighted)
            );
        })
        .assert_ok();
}

fn check_future_safe_state<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    from: u64,
    to: u64,
    num_obs: u64,
    first_reserve_last_obs: u64,
    second_reserve_last_obs: u64,
    first_reserve_weighted: u64,
    second_reserve_weighted: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pair_setup
        .blockchain_wrapper
        .execute_query(&pair_setup.pair_wrapper, |sc| {
            let state = sc.get_future_state_or_default();

            assert_eq!(state.first_obs_block, from);
            assert_eq!(state.last_obs_block, to);
            assert_eq!(state.num_observations, num_obs);
            assert_eq!(
                state.first_token_reserve_last_obs,
                managed_biguint!(first_reserve_last_obs)
            );
            assert_eq!(
                state.second_token_reserve_last_obs,
                managed_biguint!(second_reserve_last_obs)
            );
            assert_eq!(
                state.first_token_reserve_weighted,
                managed_biguint!(first_reserve_weighted)
            );
            assert_eq!(
                state.second_token_reserve_weighted,
                managed_biguint!(second_reserve_weighted)
            );
        })
        .assert_ok();
}

#[test]
fn test_pair_setup() {
    let _ = setup_pair(pair::contract_obj);
}

#[test]
fn test_add_liquidity() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );
}

#[test]
fn test_swap_fixed_input() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );

    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        996,
    );
}

#[test]
fn test_safe_price() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(11);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        996,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        11,
        1,
        1_001_000,
        1_001_000,
        1_001_000,
        1_001_000,
    );
    check_future_safe_state(
        &mut pair_setup,
        0, /* for rust format */
        0,
        0,
        0,
        0,
        0,
        0,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(20);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        994,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        20,
        2,
        1_002_000,
        1_000_004,
        1_001_000,
        1_001_000,
    );
    check_future_safe_state(
        &mut pair_setup,
        0, /* for rust format */
        0,
        0,
        0,
        0,
        0,
        0,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(30);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        992,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        30,
        3,
        1_003_000,
        999_010,
        1_001_500,
        1_000_502,
    );
    check_future_safe_state(
        &mut pair_setup,
        0, /* for rust format */
        0,
        0,
        0,
        0,
        0,
        0,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(40);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        990,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        40,
        4,
        1_004_000,
        998_018,
        1_002_000,
        1_000_004,
    );
    check_future_safe_state(
        &mut pair_setup,
        0, /* for rust format */
        0,
        0,
        0,
        0,
        0,
        0,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(50);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        988,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        50,
        5,
        1_005_000,
        997_028,
        1_002_500,
        999_507,
    );
    check_future_safe_state(
        &mut pair_setup,
        0, /* for rust format */
        0,
        0,
        0,
        0,
        0,
        0,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(60);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        986,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        60,
        6,
        1_006_000,
        996_040,
        1_003_000,
        999_011,
    );
    check_future_safe_state(
        &mut pair_setup,
        60,
        60,
        1,
        1_006_000,
        996_040,
        1_006_000,
        996_040,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(70);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        984,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        70,
        7,
        1_007_000,
        995_054,
        1_003_500,
        998_515,
    );
    check_future_safe_state(
        &mut pair_setup,
        60,
        70,
        2,
        1_007_000,
        995_054,
        1_006_000,
        996_040,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(80);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        982,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(90);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        980,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(100);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        978,
    );
    check_current_safe_state(
        &mut pair_setup,
        11,
        100,
        10,
        1_010_000,
        992_108,
        1_005_000,
        997_032,
    );
    check_future_safe_state(
        &mut pair_setup,
        60,
        100,
        5,
        1_010_000,
        992_108,
        1_007_462,
        994_598,
    );

    pair_setup.blockchain_wrapper.set_block_nonce(110);
    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        900,
        976,
    );
    check_current_safe_state(
        &mut pair_setup,
        60,
        110,
        6,
        1_011_000,
        991_130,
        1_007_959,
        994_109,
    );
    check_future_safe_state(
        &mut pair_setup,
        110,
        110,
        1,
        1_011_000,
        991_130,
        1_011_000,
        991_130,
    );
}

#[test]
fn test_swap_protect() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );

    let protect_until_block = 10;
    let max_volume_percent = 10_000;
    let max_num_swaps = 2;
    set_swap_protect(
        &mut pair_setup,
        protect_until_block,
        max_volume_percent,
        max_num_swaps,
    );

    swap_fixed_input_expect_error(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        500_000,
        MEX_TOKEN_ID,
        1,
        "swap amount in too large",
    );

    swap_fixed_input(&mut pair_setup, WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 1, 996);
    swap_fixed_input(&mut pair_setup, WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 1, 994);

    swap_fixed_input_expect_error(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        1,
        "too many swaps by address",
    );

    pair_setup
        .blockchain_wrapper
        .set_block_nonce(protect_until_block + 1);

    swap_fixed_input(
        &mut pair_setup,
        WEGLD_TOKEN_ID,
        500_000,
        MEX_TOKEN_ID,
        1,
        331_672,
    );
}

#[test]
fn test_locked_asset() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );

    let roles = [EsdtLocalRole::NftCreate, EsdtLocalRole::NftBurn];
    pair_setup.blockchain_wrapper.set_esdt_local_roles(
        pair_setup.pair_wrapper.address_ref(),
        LKWEGLD_TOKEN_ID,
        &roles[..],
    );

    pair_setup
        .blockchain_wrapper
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_locked_asset_token_id_first(managed_token_id!(LKWEGLD_TOKEN_ID));
                sc.set_locked_asset_generate_epoch_limit(10);
                sc.set_locking_period_in_epochs(10);
            },
        )
        .assert_ok();

    pair_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &MEX_TOKEN_ID,
            0,
            &rust_biguint!(1_000),
            |sc| {
                let ret = sc.swap_tokens_fixed_input(
                    managed_token_id!(MEX_TOKEN_ID),
                    0,
                    managed_biguint!(1_000),
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_biguint!(10),
                    OptionalValue::None,
                );

                assert_eq!(ret.token_identifier, managed_token_id!(LKWEGLD_TOKEN_ID));
                assert_eq!(ret.token_nonce, 1);
                assert_eq!(ret.amount, managed_biguint!(1));
            },
        )
        .assert_ok();

    pair_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &LKWEGLD_TOKEN_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                let _ =
                    sc.unlock_assets(managed_token_id!(LKWEGLD_TOKEN_ID), 1, managed_biguint!(1));
            },
        )
        .assert_user_error(&String::from_utf8(ERROR_UNLOCK_CALLED_TOO_EARLY.to_vec()).unwrap());

    pair_setup.blockchain_wrapper.set_block_epoch(20);

    pair_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &LKWEGLD_TOKEN_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                let ret =
                    sc.unlock_assets(managed_token_id!(LKWEGLD_TOKEN_ID), 1, managed_biguint!(1));

                assert_eq!(ret.token_identifier, managed_token_id!(WEGLD_TOKEN_ID));
                assert_eq!(ret.token_nonce, 0);
                assert_eq!(ret.amount, managed_biguint!(996));
            },
        )
        .assert_ok();
}
