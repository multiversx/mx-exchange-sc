use elrond_wasm::types::BoxedBytes;
use elrond_wasm_debug::managed_biguint;
use elrond_wasm_debug::{managed_token_id, rust_biguint, DebugApi};
use pair_mock::*;
use price_discovery::redeem_token::*;
use price_discovery::PriceDiscovery;
use price_discovery::{common_storage::*, MIN_PRICE_PRECISION};

mod tests_common;
use tests_common::*;

#[test]
fn test_init() {
    let _ = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
}

#[test]
fn test_deposit_launched_tokens_ok() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );

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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );

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

pub fn user_deposit_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
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
        &(),
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
        &(),
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
}

#[test]
fn try_deposit_below_min_price() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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

    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt)
        .assert_user_error("Launched token below min price");
}

#[test]
fn deposit_above_min_price() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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

pub fn withdraw_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    penalty_percentage: u64,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let first_user_address = pd_setup.first_user_address.clone();
    let balance_before = rust_biguint!(0);
    let deposit_amt = rust_biguint!(1_000_000_000);
    let withdraw_amt = rust_biguint!(400_000_000);
    call_withdraw(pd_setup, &first_user_address, &withdraw_amt).assert_ok();

    let penalty_amount = &withdraw_amt * &penalty_percentage / MAX_PERCENTAGE;
    let withdrawn_amount = &withdraw_amt - &penalty_amount;

    pd_setup.blockchain_wrapper.check_nft_balance(
        &first_user_address,
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &(&deposit_amt - &withdraw_amt),
        &(),
    );

    // check that the SC burned the tokens
    // 1 remains for ESDTNFTAddQuantity purposes
    pd_setup.blockchain_wrapper.check_nft_balance(
        &pd_setup.pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(1),
        &(),
    );

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        ACCEPTED_TOKEN_ID,
        &(&balance_before + &withdrawn_amount),
    );

    let sc_balance_before = rust_biguint!(1_500_000_000);
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &pd_setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &(&sc_balance_before - &withdrawn_amount),
    );
}

#[test]
fn withdraw_ok() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);
}

#[test]
fn withdraw_linear_penalty_start() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);

    let linear_penalty_start_block = START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(linear_penalty_start_block);
    withdraw_ok_steps(&mut pd_setup, MIN_PENALTY_PERCENTAGE);
}

#[test]
fn withdraw_linear_penalty_end() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
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
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);

    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK + 1);

    let caller_addr = pd_setup.first_user_address.clone();
    call_withdraw(&mut pd_setup, &caller_addr, &rust_biguint!(1_000))
        .assert_user_error("Withdraw not allowed in this phase");
}

#[test]
fn create_pool_too_early() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(&mut pd_setup, &first_user_address)
        .assert_user_error("Deposit period has not ended");
}

fn create_pool_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(pd_setup, &first_user_address).assert_ok();
}

#[test]
fn create_pool_ok() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);
    create_pool_ok_steps(&mut pd_setup);

    let b_mock = &mut pd_setup.blockchain_wrapper;
    let expected_lp_token_balance = 1_100_000_000 - MINIMUM_LIQUIDITY;
    b_mock.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(expected_lp_token_balance),
    );

    b_mock
        .execute_query(&pd_setup.pd_wrapper, |sc| {
            assert_eq!(sc.lp_token_id().get(), managed_token_id!(LP_TOKEN_ID));
            assert_eq!(
                sc.total_lp_tokens_received().get(),
                managed_biguint!(expected_lp_token_balance)
            );
        })
        .assert_ok();
}

#[test]
fn try_create_pool_twice() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);
    create_pool_ok_steps(&mut pd_setup);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(&mut pd_setup, &first_user_address)
        .assert_user_error("Pool already created");
}

#[test]
fn redeem_before_pool_created() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    pd_setup.blockchain_wrapper.set_block_nonce(END_BLOCK + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(600_000_000),
    )
    .assert_user_error("Liquidity Pool not created yet");
}

#[test]
fn redeem_ok() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    pd_setup.blockchain_wrapper.set_block_epoch(5);
    create_pool_ok_steps(&mut pd_setup);

    let total_lp_tokens = 1_100_000_000 - MINIMUM_LIQUIDITY;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(total_lp_tokens),
    );

    pd_setup.blockchain_wrapper.set_block_epoch(12);

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

    let accepted_token_final_amount =
        &first_user_redeem_token_amount + &second_user_redeem_token_amount;
    let launched_token_final_amount = rust_biguint!(5_000_000_000);

    let first_user_expected_lp_tokens_balance = first_user_redeem_token_amount
        * total_lp_tokens.clone()
        / accepted_token_final_amount.clone()
        / 2u64;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        LP_TOKEN_ID,
        &first_user_expected_lp_tokens_balance,
    );
    println!(
        "First user LP tokens: {}",
        first_user_expected_lp_tokens_balance
    );

    let second_user_expected_lp_tokens_balance =
        second_user_redeem_token_amount * total_lp_tokens / accepted_token_final_amount / 2u64;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &second_user_address,
        LP_TOKEN_ID,
        &second_user_expected_lp_tokens_balance,
    );
    println!(
        "Second user LP tokens: {}",
        second_user_expected_lp_tokens_balance
    );

    let total_launched_tokens = rust_biguint!(5_000_000_000);
    let owner_expected_lp_tokens_balance =
        total_launched_tokens.clone() * total_lp_tokens / launched_token_final_amount / 2u64;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &owner_address,
        LP_TOKEN_ID,
        &owner_expected_lp_tokens_balance,
    );
    println!("Owner LP tokens: {}", owner_expected_lp_tokens_balance);

    let dust = total_lp_tokens
        - first_user_expected_lp_tokens_balance
        - second_user_expected_lp_tokens_balance
        - owner_expected_lp_tokens_balance;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &dust,
    );
    println!("Dust LP tokens: {}", dust);

    /*
        First user LP tokens: 299999727
        Second user LP tokens: 249999772
        Owner LP tokens: 549999500
        Dust LP tokens: 1
    */
}

#[test]
fn redeem_too_early() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup, 0);

    pd_setup.blockchain_wrapper.set_block_epoch(5);
    create_pool_ok_steps(&mut pd_setup);

    let first_user_address = pd_setup.first_user_address.clone();
    let first_user_redeem_token_amount = rust_biguint!(600_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_user_error("Unbond period not finished yet");
}

pub fn redeem_with_extra_tokens_from_penalties_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    user_deposit_ok_steps(pd_setup);

    let fixed_penalty_start_block =
        START_BLOCK + NO_LIMIT_PHASE_DURATION_BLOCKS + LINEAR_PENALTY_PHASE_DURATION_BLOCKS;
    pd_setup
        .blockchain_wrapper
        .set_block_nonce(fixed_penalty_start_block);

    withdraw_ok_steps(pd_setup, FIXED_PENALTY_PERCENTAGE);

    pd_setup.blockchain_wrapper.set_block_epoch(5);
    create_pool_ok_steps(pd_setup);

    let total_lp_tokens = 1_199_999_000u64;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(total_lp_tokens),
    );

    pd_setup.blockchain_wrapper.set_block_epoch(12);

    let first_user_address = pd_setup.first_user_address.clone();
    let first_user_redeem_token_amount = rust_biguint!(600_000_000);
    call_redeem(
        pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_ok();

    let second_user_address = pd_setup.second_user_address.clone();
    let second_user_redeem_token_amount = rust_biguint!(500_000_000);
    call_redeem(
        pd_setup,
        &second_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_user_redeem_token_amount,
    )
    .assert_ok();

    let owner_address = pd_setup.owner_address.clone();
    let owner_redeem_amount = rust_biguint!(5_000_000_000);
    call_redeem(
        pd_setup,
        &owner_address,
        LAUNCHED_TOKEN_REDEEM_NONCE,
        &owner_redeem_amount,
    )
    .assert_ok();

    let accepted_token_final_amount =
        &first_user_redeem_token_amount + &second_user_redeem_token_amount;
    let first_user_expected_lp_tokens_balance = first_user_redeem_token_amount
        * total_lp_tokens.clone()
        / accepted_token_final_amount.clone()
        / 2u64;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        LP_TOKEN_ID,
        &first_user_expected_lp_tokens_balance,
    );
    println!(
        "First user LP tokens: {}",
        first_user_expected_lp_tokens_balance
    );

    let second_user_expected_lp_tokens_balance = rust_biguint!(272_727_045);
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &second_user_address,
        LP_TOKEN_ID,
        &second_user_expected_lp_tokens_balance,
    );
    println!(
        "Second user LP tokens: {}",
        second_user_expected_lp_tokens_balance
    );

    let owner_expected_lp_tokens_balance = rust_biguint!(599_999_500);
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &owner_address,
        LP_TOKEN_ID,
        &owner_expected_lp_tokens_balance,
    );
    println!("Owner LP tokens: {}", owner_expected_lp_tokens_balance);

    let dust = total_lp_tokens
        - first_user_expected_lp_tokens_balance
        - second_user_expected_lp_tokens_balance
        - owner_expected_lp_tokens_balance;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &dust,
    );
    println!("Dust LP tokens: {}", dust);
}

#[test]
fn redeem_with_extra_tokens_from_penalties() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    redeem_with_extra_tokens_from_penalties_steps(&mut pd_setup);
}

#[test]
fn redeem_with_extra_rewards() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        EXTRA_REWARDS_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );
    call_deposit_extra_rewards(&mut pd_setup);
    redeem_with_extra_tokens_from_penalties_steps(&mut pd_setup);

    pd_setup
        .blockchain_wrapper
        .check_egld_balance(&pd_setup.first_user_address, &rust_biguint!(27_272_727));

    pd_setup
        .blockchain_wrapper
        .check_egld_balance(&pd_setup.second_user_address, &rust_biguint!(22_727_272));

    pd_setup
        .blockchain_wrapper
        .check_egld_balance(&pd_setup.owner_address, &rust_biguint!(50_000_000));
}

#[test]
fn extra_rewards_token_same_as_launched_token() {
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        LAUNCHED_TOKEN_ID,
        0,
        OWNER_EGLD_BALANCE,
    );

    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper
        .execute_esdt_transfer(
            &pd_setup.owner_address,
            &pd_setup.pd_wrapper,
            LAUNCHED_TOKEN_ID,
            0,
            &rust_biguint!(OWNER_EGLD_BALANCE),
            |sc| {
                sc.deposit_extra_rewards();
            },
        )
        .assert_ok();

    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    call_deposit_initial_tokens(&mut pd_setup, &rust_biguint!(5_000_000_000));

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt).assert_ok();

    // second user deposit
    let second_user_address = pd_setup.second_user_address.clone();
    let second_deposit_amt = rust_biguint!(500_000_000);
    call_deposit(&mut pd_setup, &second_user_address, &second_deposit_amt).assert_ok();

    // create pool
    pd_setup.blockchain_wrapper.set_block_epoch(5);
    create_pool_ok_steps(&mut pd_setup);

    let total_lp_tokens = 1_500_000_000 - MINIMUM_LIQUIDITY;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(total_lp_tokens),
    );

    pd_setup.blockchain_wrapper.set_block_epoch(12);

    // redeem
    let first_user_redeem_token_amount = rust_biguint!(1_000_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_ok();

    let second_user_redeem_token_amount = rust_biguint!(500_000_000);
    call_redeem(
        &mut pd_setup,
        &second_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_user_redeem_token_amount,
    )
    .assert_ok();

    // check first user balance

    // total_lp_tokens * redeem_token_amount / redeem_token_supply / 2;
    // 1_500_000_000 * 1_000_000_000 / 1_500_000_000 / 2 = 500_000_000 ~= 499_999_666 due to approximations
    let first_user_expected_lp_tokens_balance = rust_biguint!(499_999_666);
    // total_extra_rewards * lp_tokens_amount / total_lp_tokens
    // 100_000_000 * 500_000_000 / 1_500_000_000 = 100_000_000 / 3 ~= 33_333_333
    let first_user_expected_extra_rewards_balance = rust_biguint!(33_333_333);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        LP_TOKEN_ID,
        &first_user_expected_lp_tokens_balance,
    );
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        LAUNCHED_TOKEN_ID,
        &first_user_expected_extra_rewards_balance,
    );

    // check second user balance

    // ~ half of what first user gained
    let second_user_expected_lp_tokens_balance = rust_biguint!(249_999_833);
    let second_user_expected_extra_rewards_balance = rust_biguint!(16_666_666);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &second_user_address,
        LP_TOKEN_ID,
        &second_user_expected_lp_tokens_balance,
    );
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &second_user_address,
        LAUNCHED_TOKEN_ID,
        &second_user_expected_extra_rewards_balance,
    );
}

#[test]
fn extra_rewards_sft() {
    let sft_token_id = &b"SOMESFT-123456"[..];
    let sft_nonce = 5;
    let mut pd_setup = init(
        price_discovery::contract_obj,
        pair_mock::contract_obj,
        sft_token_id,
        sft_nonce,
        OWNER_EGLD_BALANCE,
    );

    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper
        .execute_esdt_transfer(
            &pd_setup.owner_address,
            &pd_setup.pd_wrapper,
            sft_token_id,
            sft_nonce,
            &rust_biguint!(OWNER_EGLD_BALANCE),
            |sc| {
                sc.deposit_extra_rewards();
            },
        )
        .assert_ok();

    // try deposit wrong nonce
    let rand_user = b_wrapper.create_user_account(&rust_biguint!(0));
    b_wrapper.set_nft_balance(
        &rand_user,
        sft_token_id,
        sft_nonce + 1,
        &rust_biguint!(500),
        &BoxedBytes::empty(),
    );
    b_wrapper
        .execute_esdt_transfer(
            &rand_user,
            &pd_setup.pd_wrapper,
            sft_token_id,
            sft_nonce + 1,
            &rust_biguint!(500),
            |sc| {
                sc.deposit_extra_rewards();
            },
        )
        .assert_user_error("Invalid payment token");

    pd_setup.blockchain_wrapper.set_block_nonce(START_BLOCK);

    call_deposit_initial_tokens(&mut pd_setup, &rust_biguint!(5_000_000_000));

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(&mut pd_setup, &first_user_address, &first_deposit_amt).assert_ok();

    // second user deposit
    let second_user_address = pd_setup.second_user_address.clone();
    let second_deposit_amt = rust_biguint!(500_000_000);
    call_deposit(&mut pd_setup, &second_user_address, &second_deposit_amt).assert_ok();

    // create pool
    pd_setup.blockchain_wrapper.set_block_epoch(5);
    create_pool_ok_steps(&mut pd_setup);

    let total_lp_tokens = 1_500_000_000 - MINIMUM_LIQUIDITY;
    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(total_lp_tokens),
    );

    pd_setup.blockchain_wrapper.set_block_epoch(12);

    // redeem
    let first_user_redeem_token_amount = rust_biguint!(1_000_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
    )
    .assert_ok();

    let second_user_redeem_token_amount = rust_biguint!(500_000_000);
    call_redeem(
        &mut pd_setup,
        &second_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_user_redeem_token_amount,
    )
    .assert_ok();

    // check first user balance

    // total_lp_tokens * redeem_token_amount / redeem_token_supply / 2;
    // 1_500_000_000 * 1_000_000_000 / 1_500_000_000 / 2 = 500_000_000 ~= 499_999_666 due to approximations
    let first_user_expected_lp_tokens_balance = rust_biguint!(499_999_666);
    // total_extra_rewards * lp_tokens_amount / total_lp_tokens
    // 100_000_000 * 500_000_000 / 1_500_000_000 = 100_000_000 / 3 ~= 33_333_333
    let first_user_expected_extra_rewards_balance = rust_biguint!(33_333_333);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &first_user_address,
        LP_TOKEN_ID,
        &first_user_expected_lp_tokens_balance,
    );
    pd_setup.blockchain_wrapper.check_nft_balance(
        &first_user_address,
        sft_token_id,
        sft_nonce,
        &first_user_expected_extra_rewards_balance,
        &BoxedBytes::empty(),
    );

    // check second user balance

    // ~ half of what first user gained
    let second_user_expected_lp_tokens_balance = rust_biguint!(249_999_833);
    let second_user_expected_extra_rewards_balance = rust_biguint!(16_666_666);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        &second_user_address,
        LP_TOKEN_ID,
        &second_user_expected_lp_tokens_balance,
    );
    pd_setup.blockchain_wrapper.check_nft_balance(
        &second_user_address,
        sft_token_id,
        sft_nonce,
        &second_user_expected_extra_rewards_balance,
        &BoxedBytes::empty(),
    );
}
