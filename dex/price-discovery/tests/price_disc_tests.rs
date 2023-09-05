#![allow(deprecated)]

use multiversx_sc::codec::Empty;
use multiversx_sc_scenario::{managed_biguint, managed_token_id_wrapped};
use multiversx_sc_scenario::{rust_biguint, DebugApi};
use price_discovery::common_storage::*;
use price_discovery::redeem_token::*;
use price_discovery::PriceDiscovery;

mod tests_common;
use simple_lock::locked_token::LockedTokenAttributes;
use tests_common::*;

const MIN_PRICE_PRECISION: u64 = 1_000_000_000_000_000_000;

#[test]
fn test_init() {
    let _ = init(price_discovery::contract_obj);
}

#[test]
fn test_deposit_launched_tokens_ok() {
    let mut pd_setup = init(price_discovery::contract_obj);

    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    let init_deposit_amt = rust_biguint!(5_000_000_000);

    call_deposit_initial_tokens(&mut pd_setup, &init_deposit_amt);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LAUNCHED_TOKEN_ID,
        &init_deposit_amt,
    );
}

#[test]
fn deposit_too_early() {
    let mut pd_setup = init(price_discovery::contract_obj);

    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK - 1);

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    call_deposit(
        &mut pd_setup,
        &first_user_address,
        &rust_biguint!(1_000_000_000),
    )
    .assert_user_error("Deposit not allowed in this phase");
}

pub fn user_deposit_ok_steps<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    call_deposit_initial_tokens(pd_setup, &rust_biguint!(5_000_000_000));

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(pd_setup, &first_user_address, &first_deposit_amt).assert_ok();

    pd_setup.blockchain_wrapper.check_nft_balance(
        &first_user_address,
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_deposit_amt,
        Some(&Empty),
    );

    // second user deposit
    let second_user_address = pd_setup.second_user_address.clone();
    let second_deposit_amt = rust_biguint!(500_000_000);
    call_deposit(pd_setup, &second_user_address, &second_deposit_amt).assert_ok();

    pd_setup.blockchain_wrapper.check_nft_balance(
        &second_user_address,
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_deposit_amt,
        Some(&Empty),
    );

    // check SC balance
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &(first_deposit_amt + second_deposit_amt),
    );
}

#[test]
fn user_deposit_ok() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
}

#[test]
fn try_deposit_below_min_price() {
    let mut pd_setup = init(price_discovery::contract_obj);
    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    let owner_addr = pd_setup.owner_address.clone();
    pd_setup
        .blockchain_wrapper
        .execute_tx(&owner_addr, &pd_setup.pd_wrapper, &rust_biguint!(0), |sc| {
            // each launched token = 0.5 accepted token
            sc.min_launched_token_price()
                .set(&managed_biguint!(MIN_PRICE_PRECISION / 2));
        })
        .assert_ok();

    call_deposit_initial_tokens(&mut pd_setup, &rust_biguint!(5_000_000_000));

    // deposit accepted tokens, even if below min price
    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt).assert_ok();

    // try deposit more launched tokens
    let b_mock = &mut pd_setup.blockchain_wrapper;
    let rand_user = b_mock.create_user_account(&rust_biguint!(0));
    b_mock.set_esdt_balance(&rand_user, LAUNCHED_TOKEN_ID, &rust_biguint!(500));

    b_mock
        .execute_esdt_transfer(
            &rand_user,
            &pd_setup.pd_wrapper,
            LAUNCHED_TOKEN_ID,
            0,
            &rust_biguint!(500),
            |sc| {
                sc.deposit();
            },
        )
        .assert_user_error("Launched token below min price");
}

#[test]
fn deposit_above_min_price() {
    let mut pd_setup = init(price_discovery::contract_obj);
    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    let owner_addr = pd_setup.owner_address.clone();
    pd_setup
        .blockchain_wrapper
        .execute_tx(&owner_addr, &pd_setup.pd_wrapper, &rust_biguint!(0), |sc| {
            // each launched token = 0.2 accepted token
            sc.min_launched_token_price()
                .set(&managed_biguint!(MIN_PRICE_PRECISION / 5));
        })
        .assert_ok();

    call_deposit_initial_tokens(&mut pd_setup, &rust_biguint!(5_000_000_000));

    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt).assert_ok();
}

#[test]
fn withdraw_below_min_price() {
    let mut pd_setup = init(price_discovery::contract_obj);
    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    let owner_addr = pd_setup.owner_address.clone();
    pd_setup
        .blockchain_wrapper
        .execute_tx(&owner_addr, &pd_setup.pd_wrapper, &rust_biguint!(0), |sc| {
            // each launched token = 0.1 accepted token
            sc.min_launched_token_price()
                .set(&managed_biguint!(MIN_PRICE_PRECISION / 10));
        })
        .assert_ok();

    call_deposit_initial_tokens(&mut pd_setup, &rust_biguint!(5_000_000_000));

    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt).assert_ok();

    call_withdraw(
        &mut pd_setup,
        &first_user_address,
        &rust_biguint!(600_000_000),
    )
    .assert_user_error("Launched token below min price");
}

pub fn withdraw_ok_steps<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
    penalty_percentage: u64,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    let first_user_address = pd_setup.first_user_address.clone();
    let balance_before = rust_biguint!(0);
    let deposit_amt = rust_biguint!(1_000_000_000);
    let withdraw_amt = rust_biguint!(400_000_000);
    call_withdraw(pd_setup, &first_user_address, &withdraw_amt).assert_ok();

    let penalty_amount = &withdraw_amt * penalty_percentage / MAX_PERCENTAGE;
    let withdrawn_amount = &withdraw_amt - &penalty_amount;

    pd_setup.blockchain_wrapper.check_nft_balance(
        &first_user_address,
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &(&deposit_amt - &withdraw_amt),
        Some(&Empty),
    );

    // check that the SC burned the tokens
    // 1 remains for ESDTNFTAddQuantity purposes
    pd_setup.blockchain_wrapper.check_nft_balance(
        pd_setup.pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(1),
        Some(&Empty),
    );

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        ACCEPTED_TOKEN_ID,
        &(&balance_before + &withdrawn_amount),
    );

    let sc_balance_before = rust_biguint!(1_500_000_000);
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &(&sc_balance_before - &withdrawn_amount),
    );
}

#[test]
fn withdraw_ok() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);
}

#[test]
fn withdraw_linear_penalty_start() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    let linear_penalty_start_block = START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(linear_penalty_start_block);
    withdraw_ok_steps(&mut pd_setup, MIN_PENALTY_PERCENTAGE);
}

#[test]
fn withdraw_linear_penalty_end() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    let linear_penalty_end_block =
        START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS + LINEAR_PENALTY_PHASE_DURATION_BLOCKS - 1;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(linear_penalty_end_block);
    withdraw_ok_steps(&mut pd_setup, MAX_PENALTY_PERCENTAGE);
}

#[test]
fn withdraw_linear_penalty_middle() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    let linear_penalty_start_block = START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS;
    let linear_penalty_end_block =
        START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS + LINEAR_PENALTY_PHASE_DURATION_BLOCKS - 1;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce((linear_penalty_start_block + linear_penalty_end_block) / 2);
    withdraw_ok_steps(
        &mut pd_setup,
        (MIN_PENALTY_PERCENTAGE + MAX_PENALTY_PERCENTAGE) / 2,
    );
}

#[test]
fn withdraw_fixed_penalty() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    let fixed_penalty_start_block =
        START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS + LINEAR_PENALTY_PHASE_DURATION_BLOCKS;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(fixed_penalty_start_block);
    withdraw_ok_steps(&mut pd_setup, FIXED_PENALTY_PERCENTAGE);
}

#[test]
fn try_deposit_in_withdraw_only_phase() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    let fixed_penalty_start_block =
        START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS + LINEAR_PENALTY_PHASE_DURATION_BLOCKS;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(fixed_penalty_start_block);

    let caller_addr = pd_setup.second_user_address.clone();
    call_deposit(&mut pd_setup, &caller_addr, &rust_biguint!(1_000))
        .assert_user_error("Deposit not allowed in this phase");
}

#[test]
fn withdraw_too_late() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK + 1);

    let caller_addr = pd_setup.first_user_address.clone();
    call_withdraw(&mut pd_setup, &caller_addr, &rust_biguint!(1_000))
        .assert_user_error("Withdraw not allowed in this phase");
}

#[test]
fn redeem_ok() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    let first_user_redeem_token_amount = rust_biguint!(600_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_ok();

    let second_user_address = pd_setup.second_user_address.clone();
    let second_user_redeem_token_amount = rust_biguint!(500_000_000);
    call_redeem(
        &mut pd_setup,
        &second_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_user_redeem_token_amount,
    )
    .assert_ok();

    let owner_address = pd_setup.owner_address.clone();
    let owner_redeem_amount = rust_biguint!(5_000_000_000);
    call_redeem(
        &mut pd_setup,
        &owner_address,
        LAUNCHED_TOKEN_REDEEM_NONCE,
        &owner_redeem_amount,
    )
    .assert_ok();

    let _ = DebugApi::dummy();
    let first_user_expected_launched_tokens_balance =
        rust_biguint!(5_000_000_000u64 * 600_000_000 / 1_100_000_000);
    pd_setup.blockchain_wrapper.check_nft_balance(
        &first_user_address,
        LOCKED_TOKEN_ID,
        1,
        &first_user_expected_launched_tokens_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(LAUNCHED_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: UNLOCK_EPOCH,
        }),
    );

    let second_user_expected_launched_tokens_balance =
        rust_biguint!(5_000_000_000u64 * 500_000_000 / 1_100_000_000);
    pd_setup.blockchain_wrapper.check_nft_balance(
        &second_user_address,
        LOCKED_TOKEN_ID,
        1,
        &second_user_expected_launched_tokens_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(LAUNCHED_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: UNLOCK_EPOCH,
        }),
    );

    let owner_expected_accepted_tokens_balance =
        rust_biguint!(1_100_000_000u64 * 5_000_000_000 / 5_000_000_000);
    pd_setup.blockchain_wrapper.check_nft_balance(
        &owner_address,
        LOCKED_TOKEN_ID,
        2,
        &owner_expected_accepted_tokens_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(ACCEPTED_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: UNLOCK_EPOCH,
        }),
    );
}

#[test]
fn redeem_too_early() {
    let mut pd_setup = init(price_discovery::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK - 1);

    let first_user_address = pd_setup.first_user_address.clone();
    let first_user_redeem_token_amount = rust_biguint!(600_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_user_error("Redeem not allowed in this phase");
}
