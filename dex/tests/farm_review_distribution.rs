use std::ops::Mul;

use elrond_wasm::storage::mappers::StorageTokenWrapper;
use elrond_wasm::types::{Address, BigUint, EsdtLocalRole, MultiValueEncoded};
use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    tx_mock::TxInputESDT, DebugApi,
};
type RustBigUint = num_bigint::BigUint;

use config::*;
use custom_rewards::*;
use farm::*;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};
use rewards::*;

const FARM_WASM_PATH: &'static str = "farm/output/farm.wasm";

const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // farming token ID
const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const MIN_FARMING_EPOCHS: u64 = 2;
const PENALTY_PERCENT: u64 = 10;

#[allow(dead_code)] // owner_address is unused, at least for now
struct FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
}

fn setup_farm<FarmObjBuilder>(
    farm_builder: FarmObjBuilder,
    per_block_reward_amount: RustBigUint,
) -> FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
    let farm_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        farm_builder,
        FARM_WASM_PATH,
    );

    // init farm contract

    blockchain_wrapper
        .execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
            let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
            let farming_token_id = managed_token_id!(LP_TOKEN_ID);
            let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
            let pair_address = managed_address!(&Address::zero());

            sc.init(
                reward_token_id,
                farming_token_id,
                division_safety_constant,
                pair_address,
                OptionalValue::None,
                MultiValueEncoded::new(),
            );

            let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
            sc.farm_token().set_token_id(&farm_token_id);

            sc.per_block_reward_amount()
                .set(&to_managed_biguint(per_block_reward_amount));
            sc.minimum_farming_epochs().set(&MIN_FARMING_EPOCHS);
            sc.penalty_percent().set(&PENALTY_PERCENT);

            sc.state().set(&State::Active);
            sc.produce_rewards_enabled().set(&true);
        })
        .assert_ok();

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    let farming_token_roles = [EsdtLocalRole::Burn];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        LP_TOKEN_ID,
        &farming_token_roles[..],
    );

    let reward_token_roles = [EsdtLocalRole::Mint];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &reward_token_roles[..],
    );

    FarmSetup {
        blockchain_wrapper,
        owner_address: owner_addr,
        farm_wrapper,
    }
}

enum Action {
    EnterFarm(Address, RustBigUint),
    ExitFarm(Address, u64, RustBigUint, RustBigUint),
    RewardPerBlockRateChange(RustBigUint),
}

struct Expected {
    reward_reserve: RustBigUint,
    reward_per_share: RustBigUint, // also known as Price Per Share (PPS)
    total_farm_supply: RustBigUint,
}

impl Expected {
    fn new(
        reward_reserve: RustBigUint,
        rewards_per_share: RustBigUint,
        total_farm_supply: RustBigUint,
    ) -> Self {
        Self {
            reward_reserve,
            reward_per_share: rewards_per_share,
            total_farm_supply,
        }
    }
}

fn enter_farm<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    caller: &Address,
    farm_in_amount: RustBigUint,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let mut payments = Vec::new();
    payments.push(TxInputESDT {
        token_identifier: LP_TOKEN_ID.to_vec(),
        nonce: 0,
        value: farm_in_amount.clone(),
    });

    let mut expected_total_out_amount = RustBigUint::default();
    for payment in payments.iter() {
        expected_total_out_amount += payment.value.clone();
    }

    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_multi_transfer(&caller, &farm_setup.farm_wrapper, &payments, |sc| {
            let payment = sc.enter_farm();
            assert_eq!(payment.token_identifier, managed_token_id!(FARM_TOKEN_ID));
            check_biguint_eq(
                payment.amount,
                expected_total_out_amount,
                "Enter farm, farm token payment mismatch.",
            );
        })
        .assert_ok();

    let mut sc_call =
        ScCallMandos::new(&caller, farm_setup.farm_wrapper.address_ref(), "enterFarm");
    sc_call.add_esdt_transfer(LP_TOKEN_ID, 0, &farm_in_amount);

    let mut tx_expect = TxExpectMandos::new(0);
    tx_expect.add_out_value(&farm_in_amount.to_bytes_be());

    b_mock.add_mandos_sc_call(sc_call, Some(tx_expect));
}

fn to_managed_biguint(value: RustBigUint) -> BigUint<DebugApi> {
    BigUint::from_bytes_be(&value.to_bytes_be())
}

fn to_rust_biguint(value: BigUint<DebugApi>) -> RustBigUint {
    RustBigUint::from_bytes_be(value.to_bytes_be().as_slice())
}

fn exit_farm<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    caller: &Address,
    farm_token_nonce: u64,
    farm_out_amount: RustBigUint,
    expected_mex_balance: RustBigUint,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            farm_token_nonce,
            &farm_out_amount.clone(),
            |sc| {
                let multi_result = sc.exit_farm();

                let (first_result, second_result) = multi_result.into_tuple();

                assert_eq!(
                    first_result.token_identifier,
                    managed_token_id!(LP_TOKEN_ID)
                );
                assert_eq!(first_result.token_nonce, 0);

                assert_eq!(
                    second_result.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(second_result.token_nonce, 0);
            },
        )
        .assert_ok();

    b_mock.check_esdt_balance(&caller, MEX_TOKEN_ID, &expected_mex_balance);
}

fn reward_per_block_rate_change<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    new_rate: RustBigUint,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    farm_setup
        .blockchain_wrapper
        .execute_tx(
            &farm_setup.owner_address,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_per_block_rewards(to_managed_biguint(new_rate));
            },
        )
        .assert_ok();
}

fn handle_action<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>, action: Action)
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    match action {
        Action::EnterFarm(caller, amount) => enter_farm(farm_setup, &caller, amount),
        Action::ExitFarm(caller, farm_token_nonce, farm_out_amount, expected_mex_balance) => {
            exit_farm(
                farm_setup,
                &caller,
                farm_token_nonce,
                farm_out_amount,
                expected_mex_balance,
            )
        }
        Action::RewardPerBlockRateChange(new_rate) => {
            reward_per_block_rate_change(farm_setup, new_rate)
        }
    }
}

fn check_biguint_eq(actual: BigUint<DebugApi>, expected: RustBigUint, message: &str) {
    assert_eq!(
        actual.clone(),
        to_managed_biguint(expected.clone()),
        "{} Expected: {}, have {}",
        message,
        expected,
        to_rust_biguint(actual),
    );
}

fn check_expected<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>, expected: Expected)
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    farm_setup
        .blockchain_wrapper
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            check_biguint_eq(
                sc.reward_reserve().get(),
                expected.reward_reserve,
                "Reward reserve mismatch.",
            );
            check_biguint_eq(
                sc.reward_per_share().get(),
                expected.reward_per_share,
                "Reward per share mismatch.",
            );
            check_biguint_eq(
                sc.farm_token_supply().get(),
                expected.total_farm_supply,
                "Total farm token supply mismatch.",
            );
        })
        .assert_ok();
}

fn step<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    block_number: u64,
    action: Action,
    expected: Expected,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    farm_setup
        .blockchain_wrapper
        .set_block_nonce(block_number + 1); // spreadsheet correction
    handle_action(farm_setup, action);
    check_expected(farm_setup, expected);
}

fn new_address_with_lp_tokens<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    amount: RustBigUint,
) -> Address
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let blockchain_wrapper = &mut farm_setup.blockchain_wrapper;
    let address = blockchain_wrapper.create_user_account(&rust_biguint!(0));
    blockchain_wrapper.set_esdt_balance(&address, LP_TOKEN_ID, &amount);
    address
}

#[test]
fn test_overview() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = setup_farm(farm::contract_obj, per_block_reward_amount);
    let alice = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    let bob = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    let eve = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(alice.clone(), rust_biguint!(1_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(1_000)),
    );
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(bob.clone(), rust_biguint!(2_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(3_000)),
    );
    step(
        &mut farm_setup,
        6,
        Action::EnterFarm(eve.clone(), rust_biguint!(500)),
        Expected::new(
            rust_biguint!(700),
            rust_biguint!(100_000_000_000),
            rust_biguint!(3_500),
        ),
    );
    step(
        &mut farm_setup,
        10,
        Action::ExitFarm(bob, 2, rust_biguint!(2_000), rust_biguint!(428)),
        Expected::new(
            rust_biguint!(672),
            rust_biguint!(214_285_714_285),
            rust_biguint!(1_500),
        ),
    );
    step(
        &mut farm_setup,
        13,
        Action::ExitFarm(alice, 1, rust_biguint!(1_000), rust_biguint!(414)),
        Expected::new(
            rust_biguint!(558),
            rust_biguint!(414_285_714_285),
            rust_biguint!(500),
        ),
    );
    step(
        &mut farm_setup,
        16,
        Action::ExitFarm(eve, 3, rust_biguint!(500), rust_biguint!(457)),
        Expected::new(
            rust_biguint!(401),
            rust_biguint!(1_014_285_714_285),
            rust_biguint!(0),
        ),
    );
}

#[test]
fn test_overview_but_changes_in_per_reward_block() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = setup_farm(farm::contract_obj, per_block_reward_amount);
    let alice = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    let bob = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    let eve = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(alice.clone(), rust_biguint!(1_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(1_000)),
    );
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(bob.clone(), rust_biguint!(2_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(3_000)),
    );
    step(
        &mut farm_setup,
        6,
        Action::EnterFarm(eve.clone(), rust_biguint!(500)),
        Expected::new(
            rust_biguint!(700),
            rust_biguint!(100_000_000_000),
            rust_biguint!(3_500),
        ),
    );
    step(
        &mut farm_setup,
        8,
        Action::RewardPerBlockRateChange(rust_biguint!(50)),
        Expected::new(
            rust_biguint!(900),
            rust_biguint!(157_142_857_142),
            rust_biguint!(3_500),
        ),
    );
    step(
        &mut farm_setup,
        10,
        Action::ExitFarm(bob, 2, rust_biguint!(2_000), rust_biguint!(371)),
        Expected::new(
            rust_biguint!(629),
            rust_biguint!(185_714_285_713),
            rust_biguint!(1_500),
        ),
    );
    step(
        &mut farm_setup,
        13,
        Action::ExitFarm(alice, 1, rust_biguint!(1_000), rust_biguint!(285)),
        Expected::new(
            rust_biguint!(494),
            rust_biguint!(285_714_285_713),
            rust_biguint!(500),
        ),
    );
    step(
        &mut farm_setup,
        16,
        Action::ExitFarm(eve, 3, rust_biguint!(500), rust_biguint!(242)),
        Expected::new(
            rust_biguint!(402),
            rust_biguint!(585_714_285_713),
            rust_biguint!(0),
        ),
    );
}

fn parse_biguint(str: &str) -> RustBigUint {
    let str_without_underscores = str.to_owned().replace("_", "");
    RustBigUint::parse_bytes(str_without_underscores.as_bytes(), 10).unwrap()
}

fn exp18(value: u64) -> RustBigUint {
    value.mul(rust_biguint!(10).pow(18))
}

#[test]
fn test_overview_realistic_numbers() {
    let per_block_reward_amount = exp18(100);
    let mut farm_setup = setup_farm(farm::contract_obj, per_block_reward_amount);
    let alice = new_address_with_lp_tokens(&mut farm_setup, exp18(5_000));
    let bob = new_address_with_lp_tokens(&mut farm_setup, exp18(5_000));
    let eve = new_address_with_lp_tokens(&mut farm_setup, exp18(5_000));
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(alice.clone(), exp18(1_000)),
        Expected::new(exp18(400), rust_biguint!(0), exp18(1_000)),
    );
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(bob.clone(), exp18(2_000)),
        Expected::new(exp18(400), rust_biguint!(0), exp18(3_000)),
    );
    step(
        &mut farm_setup,
        6,
        Action::EnterFarm(eve.clone(), exp18(500)),
        Expected::new(exp18(700), rust_biguint!(100_000_000_000), exp18(3_500)),
    );
    step(
        &mut farm_setup,
        10,
        Action::ExitFarm(
            bob,
            2,
            exp18(2_000),
            parse_biguint("428_571_428_570_000_000_000"),
        ),
        Expected::new(
            parse_biguint("671_428_571_430_000_000_000"),
            rust_biguint!(214_285_714_285),
            exp18(1_500),
        ),
    );
    step(
        &mut farm_setup,
        13,
        Action::ExitFarm(
            alice,
            1,
            exp18(1_000),
            parse_biguint("414_285_714_285_000_000_000"),
        ),
        Expected::new(
            parse_biguint("557_142_857_145_000_000_000"),
            rust_biguint!(414_285_714_285),
            exp18(500),
        ),
    );
    step(
        &mut farm_setup,
        16,
        Action::ExitFarm(
            eve,
            3,
            exp18(500),
            parse_biguint("457_142_857_142_500_000_000"),
        ),
        Expected::new(
            parse_biguint("400_000_000_002_500_000_000"),
            rust_biguint!(1_014_285_714_285),
            exp18(0),
        ),
    );
}

fn exp21(value: u64) -> RustBigUint {
    value.mul(rust_biguint!(10).pow(21))
}

#[test]
fn test_billion_to_trillion() {
    let per_block_reward_amount = exp21(100);
    let mut farm_setup = setup_farm(farm::contract_obj, per_block_reward_amount);
    let alice = new_address_with_lp_tokens(&mut farm_setup, exp21(5_000));
    let bob = new_address_with_lp_tokens(&mut farm_setup, exp21(5_000));
    let eve = new_address_with_lp_tokens(&mut farm_setup, exp21(5_000));
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(alice.clone(), exp21(1_000)),
        Expected::new(exp21(400), rust_biguint!(0), exp21(1_000)),
    );
    step(
        &mut farm_setup,
        3,
        Action::EnterFarm(bob.clone(), exp21(2_000)),
        Expected::new(exp21(400), rust_biguint!(0), exp21(3_000)),
    );
    step(
        &mut farm_setup,
        6,
        Action::EnterFarm(eve.clone(), exp21(500)),
        Expected::new(exp21(700), rust_biguint!(100_000_000_000), exp21(3_500)),
    );
    step(
        &mut farm_setup,
        10,
        Action::ExitFarm(
            bob,
            2,
            exp21(2_000),
            parse_biguint("428_571_428_570_000_000_000_000"),
        ),
        Expected::new(
            parse_biguint("671_428_571_430_000_000_000_000"),
            rust_biguint!(214_285_714_285),
            exp21(1_500),
        ),
    );
    step(
        &mut farm_setup,
        13,
        Action::ExitFarm(
            alice,
            1,
            exp21(1_000),
            parse_biguint("414_285_714_285_000_000_000_000"),
        ),
        Expected::new(
            parse_biguint("557_142_857_145_000_000_000_000"),
            rust_biguint!(414_285_714_285),
            exp21(500),
        ),
    );
    step(
        &mut farm_setup,
        16,
        Action::ExitFarm(
            eve,
            3,
            exp21(500),
            parse_biguint("457_142_857_142_500_000_000_000"),
        ),
        Expected::new(
            parse_biguint("400_000_000_002_500_000_000_000"),
            rust_biguint!(1_014_285_714_285),
            exp21(0),
        ),
    );
}

#[test]
fn test_rv_earn_twice() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = setup_farm(farm::contract_obj, per_block_reward_amount);
    let alice = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    let bob = new_address_with_lp_tokens(&mut farm_setup, rust_biguint!(5_000));
    step(
        &mut farm_setup,
        1,
        Action::EnterFarm(alice.clone(), rust_biguint!(100)),
        Expected::new(rust_biguint!(200), rust_biguint!(0), rust_biguint!(100)),
    );
    step(
        &mut farm_setup,
        2,
        Action::EnterFarm(bob.clone(), rust_biguint!(100)),
        Expected::new(
            rust_biguint!(300),
            rust_biguint!(1_000_000_000_000),
            rust_biguint!(200),
        ),
    );
    step(
        &mut farm_setup,
        9,
        Action::ExitFarm(alice, 1, rust_biguint!(100), rust_biguint!(450)),
        Expected::new(
            rust_biguint!(550),
            rust_biguint!(4_500_000_000_000),
            rust_biguint!(100),
        ),
    );
    step(
        &mut farm_setup,
        9,
        Action::ExitFarm(bob, 2, rust_biguint!(100), rust_biguint!(350)),
        Expected::new(
            rust_biguint!(200),
            rust_biguint!(4_500_000_000_000),
            rust_biguint!(0),
        ),
    );
}
