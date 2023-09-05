#![allow(deprecated)]

use multiversx_sc::codec::multi_types::{MultiValue3, OptionalValue};
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

use farm::exit_penalty::ExitPenaltyModule;
use pair::config as pair_config;
use pair::safe_price_view::{SafePriceViewModule, DEFAULT_SAFE_PRICE_ROUNDS_OFFSET};
use pair::*;
use pair_config::ConfigModule as _;
use pausable::{PausableModule, State};

use ::config as farm_config;
use farm::*;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_config::ConfigModule as _;
use farm_token::FarmTokenModule;

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
            let total_fee_percent = 300u64;
            let special_fee_percent = 50u64;

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                total_fee_percent,
                special_fee_percent,
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );

            let lp_token_id = managed_token_id!(LP_TOKEN_ID);
            sc.lp_token_identifier().set(&lp_token_id);

            sc.state().set(pausable::State::Active);
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

    let mut block_round = 1;
    b_mock.set_block_round(block_round);
    b_mock.set_block_nonce(BLOCK_NONCE_FIRST_ADD_LIQ);

    let temp_user_addr = b_mock.create_user_account(&rust_zero);
    b_mock.set_esdt_balance(
        &temp_user_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_WEGLD_TOKENS * 2),
    );
    b_mock.set_esdt_balance(
        &temp_user_addr,
        RIDE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS * 2),
    );

    add_liquidity(
        &temp_user_addr,
        b_mock,
        &pair_wrapper,
        1_001_000_000,
        1_000_000_000,
        1_001_000_000,
        1_000_000_000,
        1_000_999_000,
        1_001_000_000,
        1_001_000_000,
    );

    block_round += 1;
    b_mock.set_block_round(block_round);
    b_mock.set_block_nonce(BLOCK_NONCE_SECOND_ADD_LIQ);

    add_liquidity(
        user_addr,
        b_mock,
        &pair_wrapper,
        1_001_000_000,
        1_000_000_000,
        1_001_000_000,
        1_000_000_000,
        USER_TOTAL_LP_TOKENS,
        1_001_000_000,
        1_001_000_000,
    );

    // Extra operations to record the new reserves
    block_round += DEFAULT_SAFE_PRICE_ROUNDS_OFFSET;
    b_mock.set_block_round(block_round);
    add_liquidity(
        &temp_user_addr,
        b_mock,
        &pair_wrapper,
        1_001_000_000,
        1_000_000_000,
        1_001_000_000,
        1_000_000_000,
        USER_TOTAL_LP_TOKENS,
        1_001_000_000,
        1_001_000_000,
    );
    // Remove liquidity to have the correct lp token supply
    remove_liquidity(&temp_user_addr, b_mock, &pair_wrapper, USER_TOTAL_LP_TOKENS);

    b_mock
        .execute_tx(user_addr, &pair_wrapper, &rust_biguint!(0), |sc| {
            sc.get_lp_tokens_safe_price_by_round_offset(
                managed_address!(pair_wrapper.address_ref()),
                1,
                managed_biguint!(1_000_000_000),
            );
        })
        .assert_ok();

    b_mock.set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP);

    pair_wrapper
}

#[allow(clippy::too_many_arguments)]
fn add_liquidity<PairObjBuilder>(
    user_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    pair_wrapper: &ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
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
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(first_token_amount),
        },
        TxTokenTransfer {
            token_identifier: RIDE_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(second_token_amount),
        },
    ];

    b_mock
        .execute_esdt_multi_transfer(user_address, pair_wrapper, &payments, |sc| {
            let MultiValue3 { 0: payments } = sc.add_liquidity(
                managed_biguint!(first_token_min),
                managed_biguint!(second_token_min),
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

            assert_eq!(
                payments.2.token_identifier,
                managed_token_id!(RIDE_TOKEN_ID)
            );
            assert_eq!(payments.2.token_nonce, 0);
            assert_eq!(payments.2.amount, managed_biguint!(expected_second_amount));
        })
        .assert_ok();
}

fn remove_liquidity<PairObjBuilder>(
    user_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    pair_wrapper: &ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    lp_token_amount: u64,
) where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    b_mock
        .execute_esdt_transfer(
            user_address,
            pair_wrapper,
            LP_TOKEN_ID,
            0,
            &rust_biguint!(lp_token_amount),
            |sc| {
                sc.remove_liquidity(
                    managed_biguint!(lp_token_amount),
                    managed_biguint!(lp_token_amount),
                );
            },
        )
        .assert_ok();
}

pub fn setup_lp_farm<FarmObjBuilder>(
    owner_addr: &Address,
    user_addr: &Address,
    b_mock: &mut BlockchainStateWrapper,
    farm_builder: FarmObjBuilder,
    user_farm_in_amount: u64,
) -> ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
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
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::new(),
            );

            let farm_token_id = managed_token_id!(LP_FARM_TOKEN_ID);
            sc.farm_token().set_token_id(farm_token_id);

            sc.minimum_farming_epochs().set(MIN_FARMING_EPOCHS);
            sc.penalty_percent().set(PENALTY_PERCENT);

            sc.state().set(State::Active);
            sc.produce_rewards_enabled().set(true);
            sc.per_block_reward_amount()
                .set(&managed_biguint!(LP_FARM_PER_BLOCK_REWARD_AMOUNT));
            sc.last_reward_block_nonce()
                .set(BLOCK_NONCE_AFTER_PAIR_SETUP);
        })
        .assert_ok();

    b_mock
        .execute_tx(owner_addr, &farm_wrapper, &rust_biguint!(0), |sc| {
            sc.set_boosted_yields_factors(
                managed_biguint!(USER_REWARDS_BASE_CONST),
                managed_biguint!(USER_REWARDS_ENERGY_CONST),
                managed_biguint!(USER_REWARDS_FARM_CONST),
                managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
            );
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

    enter_farm(user_addr, b_mock, &farm_wrapper, user_farm_in_amount, &[]);

    farm_wrapper
}

fn enter_farm<FarmObjBuilder>(
    user_address: &Address,
    b_mock: &mut BlockchainStateWrapper,
    farm_wrapper: &ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    farm_in_amount: u64,
    additional_farm_tokens: &[TxTokenTransfer],
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
    payments.push(TxTokenTransfer {
        token_identifier: LP_TOKEN_ID.to_vec(),
        nonce: 0,
        value: rust_biguint!(farm_in_amount),
    });
    payments.extend_from_slice(additional_farm_tokens);

    let mut expected_total_out_amount = 0;
    for payment in payments.iter() {
        expected_total_out_amount += payment.value.to_u64_digits()[0];
    }

    b_mock
        .execute_esdt_multi_transfer(user_address, farm_wrapper, &payments, |sc| {
            let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
            let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
            assert_eq!(
                out_farm_token.token_identifier,
                managed_token_id!(LP_FARM_TOKEN_ID)
            );
            assert_eq!(
                out_farm_token.amount,
                managed_biguint!(expected_total_out_amount)
            );
        })
        .assert_ok();
}
