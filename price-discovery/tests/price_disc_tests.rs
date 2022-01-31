use elrond_wasm::types::TokenIdentifier;
use elrond_wasm_debug::{managed_biguint, testing_framework::*};
use elrond_wasm_debug::{managed_token_id, rust_biguint, DebugApi};
use pair_mock::*;
use price_discovery::common_storage::*;
use price_discovery::redeem_token::*;

mod tests_common;
use tests_common::*;

#[test]
fn test_init() {
    let _ = init(price_discovery::contract_obj, pair_mock::contract_obj);
}

#[test]
fn test_deposit_launched_tokens_ok() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);

    pd_setup.blockchain_wrapper.set_block_epoch(START_EPOCH + 1);

    let init_deposit_amt = rust_biguint!(5_000_000_000);

    call_deposit_initial_tokens(&mut pd_setup, &init_deposit_amt, StateChange::Commit);

    pd_setup.blockchain_wrapper.check_esdt_balance(
        pd_setup.pd_wrapper.address_ref(),
        LAUNCHED_TOKEN_ID,
        &init_deposit_amt,
    );
}

#[test]
fn deposit_too_early() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);

    pd_setup.blockchain_wrapper.set_block_epoch(START_EPOCH - 1);

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    call_deposit(
        &mut pd_setup,
        &first_user_address,
        &rust_biguint!(1_000_000_000),
        StateChange::Revert,
    )
    .assert_user_error("Deposit period not started yet");
}

pub fn user_deposit_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    pd_setup.blockchain_wrapper.set_block_epoch(START_EPOCH + 1);

    call_deposit_initial_tokens(pd_setup, &rust_biguint!(5_000_000_000), StateChange::Commit);

    // must clone, as we can't borrow pd_setup as mutable and as immutable at the same time
    let first_user_address = pd_setup.first_user_address.clone();
    let first_deposit_amt = rust_biguint!(1_000_000_000);
    call_deposit(
        pd_setup,
        &first_user_address,
        &first_deposit_amt,
        StateChange::Commit,
    )
    .assert_ok();

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
    call_deposit(
        pd_setup,
        &second_user_address,
        &second_deposit_amt,
        StateChange::Commit,
    )
    .assert_ok();

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
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
}

pub fn withdraw_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let first_user_address = pd_setup.first_user_address.clone();
    let balance_before = rust_biguint!(0);
    let deposit_amt = rust_biguint!(1_000_000_000);
    let withdraw_amt = rust_biguint!(400_000_000);
    call_withdraw(
        pd_setup,
        &first_user_address,
        &withdraw_amt,
        StateChange::Commit,
    )
    .assert_ok();

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
        &(&balance_before + &withdraw_amt),
    );

    let sc_balance_before = rust_biguint!(1_500_000_000);
    pd_setup.blockchain_wrapper.check_esdt_balance(
        &pd_setup.pd_wrapper.address_ref(),
        ACCEPTED_TOKEN_ID,
        &(&sc_balance_before - &withdraw_amt),
    );
}

#[test]
fn withdraw_ok() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);
}

#[test]
fn withdraw_too_late() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);

    pd_setup.blockchain_wrapper.set_block_epoch(END_EPOCH + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    let withdraw_amt = rust_biguint!(400_000_000);
    call_withdraw(
        &mut pd_setup,
        &first_user_address,
        &withdraw_amt,
        StateChange::Revert,
    )
    .assert_user_error("Deposit period ended");
}

#[test]
fn create_pool_too_early() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(&mut pd_setup, &first_user_address, StateChange::Revert)
        .assert_user_error("Deposit period has not ended");
}

fn create_pool_ok_steps<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    pd_setup.blockchain_wrapper.set_block_epoch(END_EPOCH + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(pd_setup, &first_user_address, StateChange::Commit).assert_ok();
}

#[test]
fn create_pool_ok() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);
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
                sc.launched_token_final_amount().get(),
                managed_biguint!(5_000_000_000)
            );
            assert_eq!(
                sc.accepted_token_final_amount().get(),
                managed_biguint!(1_100_000_000)
            );
            assert_eq!(
                sc.total_lp_tokens_received().get(),
                managed_biguint!(expected_lp_token_balance)
            );
        })
        .assert_ok();
}

#[test]
fn try_create_pool_twice() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);
    create_pool_ok_steps(&mut pd_setup);

    let first_user_address = pd_setup.first_user_address.clone();
    call_create_dex_liquidity_pool(&mut pd_setup, &first_user_address, StateChange::Commit)
        .assert_user_error("Pool already created");
}

#[test]
fn redeem_before_pool_created() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);

    pd_setup.blockchain_wrapper.set_block_epoch(END_EPOCH + 1);

    let first_user_address = pd_setup.first_user_address.clone();
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(600_000_000),
        StateChange::Revert,
    )
    .assert_user_error("Pool not created yet");
}

#[test]
fn redeem_ok() {
    let mut pd_setup = init(price_discovery::contract_obj, pair_mock::contract_obj);
    user_deposit_ok_steps(&mut pd_setup);
    withdraw_ok_steps(&mut pd_setup);
    create_pool_ok_steps(&mut pd_setup);

    let first_user_address = pd_setup.first_user_address.clone();
    let first_user_redeem_token_amount = rust_biguint!(600_000_000);
    call_redeem(
        &mut pd_setup,
        &first_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &first_user_redeem_token_amount,
        StateChange::Commit,
    )
    .assert_ok();

    let second_user_address = pd_setup.second_user_address.clone();
    let second_user_redeem_token_amount = rust_biguint!(500_000_000);
    call_redeem(
        &mut pd_setup,
        &second_user_address,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &second_user_redeem_token_amount,
        StateChange::Commit,
    )
    .assert_ok();

    let owner_address = pd_setup.owner_address.clone();
    let owner_redeem_amount = rust_biguint!(5_000_000_000);
    call_redeem(
        &mut pd_setup,
        &owner_address,
        LAUNCHED_TOKEN_REDEEM_NONCE,
        &owner_redeem_amount,
        StateChange::Commit,
    )
    .assert_ok();

    let total_lp_tokens = 1_100_000_000 - MINIMUM_LIQUIDITY;
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
}
