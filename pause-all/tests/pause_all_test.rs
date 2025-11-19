#![allow(deprecated)]

use farm::Farm;
use multiversx_sc::types::{Address, ManagedAddress, MultiValueEncoded, OperationCompletionStatus};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};
use pair::Pair;
use pausable::{PausableModule, State};
use pause_all::*;

static REWARD_TOKEN_ID: &[u8] = b"REWARD-123456";
static FARMING_TOKEN_ID: &[u8] = b"FARMING-123456";
const DIV_SAFETY: u64 = 1_000_000_000_000_000_000;

static FIRST_TOKEN_ID: &[u8] = FARMING_TOKEN_ID;
static SECOND_TOKEN_ID: &[u8] = b"BEST-123456";
static TOTAL_FEE_PERCENT: u64 = 50;
static SPECIAL_FEE_PERCENT: u64 = 50;

#[test]
fn pause_all_test() {
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner_address = b_mock.create_user_account(&rust_zero);
    let pause_sc = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        pause_all::contract_obj,
        "output/pause-all.wasm",
    );
    let farm_sc = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        farm::contract_obj,
        "output/farm.wasm",
    );
    let pair_sc = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        pair::contract_obj,
        "output/pair.wasm",
    );

    // init farm
    b_mock
        .execute_tx(&owner_address, &farm_sc, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(REWARD_TOKEN_ID),
                managed_token_id!(FARMING_TOKEN_ID),
                managed_biguint!(DIV_SAFETY),
                managed_address!(pair_sc.address_ref()),
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::new(),
            );

            let mut pause_whitelist =
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new();
            pause_whitelist.push(managed_address!(pause_sc.address_ref()));
            sc.add_to_pause_whitelist(pause_whitelist);

            assert_eq!(sc.state().get(), State::Inactive);
        })
        .assert_ok();

    // init pair
    b_mock
        .execute_tx(&owner_address, &pair_sc, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(FIRST_TOKEN_ID),
                managed_token_id!(SECOND_TOKEN_ID),
                managed_address!(&Address::zero()),
                managed_address!(&owner_address),
                TOTAL_FEE_PERCENT,
                SPECIAL_FEE_PERCENT,
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );

            let mut pause_whitelist =
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new();
            pause_whitelist.push(managed_address!(pause_sc.address_ref()));
            sc.add_to_pause_whitelist(pause_whitelist);

            assert_eq!(sc.state().get(), State::Inactive);
        })
        .assert_ok();

    // init pause sc
    b_mock
        .execute_tx(&owner_address, &pause_sc, &rust_zero, |sc| {
            sc.init();

            let mut pausable_contracts = MultiValueEncoded::new();
            pausable_contracts.push(managed_address!(farm_sc.address_ref()));
            pausable_contracts.push(managed_address!(pair_sc.address_ref()));
            sc.add_pausable_contracts(pausable_contracts);
        })
        .assert_ok();

    // resume farm and pair (initially paused)
    b_mock
        .execute_tx(&owner_address, &pause_sc, &rust_zero, |sc| {
            let run_result = sc.resume_all();
            assert_eq!(run_result, OperationCompletionStatus::Completed);
        })
        .assert_ok();

    b_mock
        .execute_query(&farm_sc, |sc| {
            assert_eq!(sc.state().get(), State::Active);
        })
        .assert_ok();

    b_mock
        .execute_query(&pair_sc, |sc| {
            assert_eq!(sc.state().get(), State::Active);
        })
        .assert_ok();

    // pause all
    b_mock
        .execute_tx(&owner_address, &pause_sc, &rust_zero, |sc| {
            let run_result = sc.pause_all();
            assert_eq!(run_result, OperationCompletionStatus::Completed);
        })
        .assert_ok();

    b_mock
        .execute_query(&farm_sc, |sc| {
            assert_eq!(sc.state().get(), State::Inactive);
        })
        .assert_ok();

    b_mock
        .execute_query(&pair_sc, |sc| {
            assert_eq!(sc.state().get(), State::Inactive);
        })
        .assert_ok();
}
