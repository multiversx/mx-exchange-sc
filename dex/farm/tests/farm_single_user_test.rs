#![allow(deprecated)]

mod farm_setup;

use config::ConfigModule;
use farm_setup::single_user_farm_setup::*;
use multiversx_sc::types::EsdtLocalRole;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use sc_whitelist_module::SCWhitelistModule;

#[test]
fn test_farm_setup() {
    let _ = SingleUserFarmSetup::new(farm::contract_obj);
}

#[test]
fn test_enter_farm() {
    let mut farm_setup = SingleUserFarmSetup::new(farm::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.enter_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);
}

#[test]
fn test_exit_farm() {
    let mut farm_setup = SingleUserFarmSetup::new(farm::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.enter_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let expected_mex_out = 10 * PER_BLOCK_REWARD_AMOUNT;
    let expected_lp_token_balance = rust_biguint!(USER_TOTAL_LP_TOKENS);
    farm_setup.exit_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_mex_out,
        farm_in_amount,
        &rust_biguint!(expected_mex_out),
        &expected_lp_token_balance,
    );
    farm_setup.check_farm_token_supply(0);
}

#[test]
fn test_exit_farm_with_penalty() {
    let mut farm_setup = SingleUserFarmSetup::new(farm::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.enter_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(1);
    farm_setup.set_block_nonce(10);

    let expected_farm_token_amount =
        farm_in_amount - farm_in_amount * PENALTY_PERCENT / MAX_PERCENT;
    let expected_mex_out = 10 * PER_BLOCK_REWARD_AMOUNT;
    let expected_lp_token_balance =
        rust_biguint!(USER_TOTAL_LP_TOKENS - farm_in_amount * PENALTY_PERCENT / MAX_PERCENT);
    farm_setup.exit_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_mex_out,
        expected_farm_token_amount,
        &rust_biguint!(expected_mex_out),
        &expected_lp_token_balance,
    );
    farm_setup.check_farm_token_supply(0);
}

#[test]
fn test_claim_rewards() {
    let mut farm_setup = SingleUserFarmSetup::new(farm::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.enter_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let expected_mex_out = 10 * PER_BLOCK_REWARD_AMOUNT;
    let expected_lp_token_balance = rust_biguint!(USER_TOTAL_LP_TOKENS - farm_in_amount);
    let expected_reward_per_share = 500_000_000;
    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_mex_out,
        &rust_biguint!(expected_mex_out),
        &expected_lp_token_balance,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);
}

fn steps_enter_farm_twice<FarmObjBuilder>(
    farm_builder: FarmObjBuilder,
) -> SingleUserFarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let mut farm_setup = SingleUserFarmSetup::new(farm_builder);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.enter_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let second_farm_in_amount = 200_000_000;
    let prev_farm_tokens = [TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: expected_farm_token_nonce,
        value: rust_biguint!(farm_in_amount),
    }];
    let current_farm_supply = farm_in_amount;

    let total_amount = farm_in_amount + second_farm_in_amount;
    let first_reward_share = 0;
    let second_reward_share =
        DIVISION_SAFETY_CONSTANT * 10 * PER_BLOCK_REWARD_AMOUNT / current_farm_supply;
    let expected_reward_per_share = (first_reward_share * farm_in_amount
        + second_reward_share * second_farm_in_amount
        + total_amount
        - 1)
        / total_amount;

    farm_setup.enter_farm(
        second_farm_in_amount,
        &prev_farm_tokens,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        5,
        0,
    );
    farm_setup.check_farm_token_supply(total_amount);

    farm_setup
}

#[test]
fn test_enter_farm_twice() {
    let _ = steps_enter_farm_twice(farm::contract_obj);
}

#[test]
fn test_exit_farm_after_enter_twice() {
    let mut farm_setup = steps_enter_farm_twice(farm::contract_obj);
    let farm_in_amount = 100_000_000;
    let second_farm_in_amount = 200_000_000;
    let total_farm_token = farm_in_amount + second_farm_in_amount;
    let expected_user_lp_balance = rust_biguint!(USER_TOTAL_LP_TOKENS);

    farm_setup.set_block_epoch(8);
    farm_setup.set_block_nonce(25);

    let current_farm_supply = farm_in_amount;

    let first_reward_share = 0;
    let second_reward_share =
        DIVISION_SAFETY_CONSTANT * 10 * PER_BLOCK_REWARD_AMOUNT / current_farm_supply;
    let prev_reward_per_share = (first_reward_share * farm_in_amount
        + second_reward_share * second_farm_in_amount
        + total_farm_token
        - 1)
        / total_farm_token;
    let new_reward_per_share = prev_reward_per_share
        + 25 * PER_BLOCK_REWARD_AMOUNT * DIVISION_SAFETY_CONSTANT / total_farm_token;
    let reward_per_share_diff = new_reward_per_share - prev_reward_per_share;

    let expected_reward_amount =
        total_farm_token * reward_per_share_diff / DIVISION_SAFETY_CONSTANT;
    farm_setup.exit_farm(
        total_farm_token,
        2,
        expected_reward_amount,
        total_farm_token,
        &rust_biguint!(expected_reward_amount),
        &expected_user_lp_balance,
    );
    farm_setup.check_farm_token_supply(0);
}

#[test]
fn test_farm_through_simple_lock() {
    use multiversx_sc::storage::mappers::StorageTokenWrapper;
    use simple_lock::locked_token::LockedTokenModule;
    use simple_lock::proxy_farm::ProxyFarmModule;
    use simple_lock::proxy_farm::*;
    use simple_lock::proxy_lp::{LpProxyTokenAttributes, ProxyLpModule};
    use simple_lock::SimpleLock;

    const LOCKED_TOKEN_ID: &[u8] = b"NOOOO-123456";
    const LOCKED_LP_TOKEN_ID: &[u8] = b"LKLP-123456";
    const FARM_PROXY_TOKEN_ID: &[u8] = b"PROXY-123456";

    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut farm_setup = SingleUserFarmSetup::new(farm::contract_obj);
    let b_mock = &mut farm_setup.blockchain_wrapper;

    // setup simple lock SC
    let lock_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&farm_setup.owner_address),
        simple_lock::contract_obj,
        "Simple Lock Path",
    );

    let farm_addr = farm_setup.farm_wrapper.address_ref().clone();
    b_mock
        .execute_tx(&farm_setup.owner_address, &lock_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LOCKED_LP_TOKEN_ID));
            sc.farm_proxy_token()
                .set_token_id(managed_token_id!(FARM_PROXY_TOKEN_ID));
            sc.add_farm_to_whitelist(
                managed_address!(&farm_addr),
                managed_token_id!(LP_TOKEN_ID),
                FarmType::SimpleFarm,
            );
        })
        .assert_ok();

    // change farming token for farm + whitelist simple lock contract
    b_mock
        .execute_tx(
            &farm_setup.owner_address,
            &farm_setup.farm_wrapper,
            &rust_zero,
            |sc| {
                sc.farming_token_id().set(&managed_token_id!(LP_TOKEN_ID));
                sc.add_sc_address_to_whitelist(managed_address!(lock_wrapper.address_ref()));
            },
        )
        .assert_ok();

    b_mock.set_esdt_local_roles(
        lock_wrapper.address_ref(),
        LOCKED_LP_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        lock_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // user lock tokens
    let user_addr = farm_setup.user_address.clone();

    let lp_proxy_token_attributes: LpProxyTokenAttributes<DebugApi> = LpProxyTokenAttributes {
        lp_token_id: managed_token_id!(LP_TOKEN_ID),
        first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
        first_token_locked_nonce: 1,
        second_token_id: managed_token_id!(MEX_TOKEN_ID),
        second_token_locked_nonce: 2,
    };

    b_mock.set_nft_balance(
        &user_addr,
        LOCKED_LP_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000_000),
        &lp_proxy_token_attributes,
    );

    b_mock.set_esdt_balance(
        lock_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(1_000_000_000),
    );

    // user enter farm
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            LOCKED_LP_TOKEN_ID,
            1,
            &rust_biguint!(1_000_000_000),
            |sc| {
                let enter_farm_result = sc.enter_farm_locked_token(FarmType::SimpleFarm);
                let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_PROXY_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, 1);
                assert_eq!(out_farm_token.amount, managed_biguint!(1_000_000_000));
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000_000),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 1,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );

    // user claim farm rewards
    b_mock.set_block_nonce(10);
    b_mock.set_block_epoch(5);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            FARM_PROXY_TOKEN_ID,
            1,
            &rust_biguint!(1_000_000_000),
            |sc| {
                let claim_result = sc.farm_claim_rewards_locked_token();
                let (new_proxy_token, reward_tokens) = claim_result.into_tuple();
                assert_eq!(
                    new_proxy_token.token_identifier,
                    managed_token_id!(FARM_PROXY_TOKEN_ID)
                );
                assert_eq!(new_proxy_token.token_nonce, 2);
                assert_eq!(new_proxy_token.amount, managed_biguint!(1_000_000_000));

                assert_eq!(
                    reward_tokens.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(reward_tokens.token_nonce, 0);
                assert_eq!(
                    reward_tokens.amount,
                    managed_biguint!(10 * PER_BLOCK_REWARD_AMOUNT)
                );
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        2,
        &rust_biguint!(1_000_000_000),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 2,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );
    b_mock.check_esdt_balance(
        &user_addr,
        MEX_TOKEN_ID,
        &rust_biguint!(10 * PER_BLOCK_REWARD_AMOUNT),
    );

    // user exit farm
    b_mock.set_block_nonce(25);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            FARM_PROXY_TOKEN_ID,
            2,
            &rust_biguint!(1_000_000_000),
            |sc| {
                let exit_farm_result = sc.exit_farm_locked_token(managed_biguint!(1_000_000_000));
                let (locked_tokens, reward_tokens) = exit_farm_result.into_tuple();

                assert_eq!(
                    locked_tokens.token_identifier,
                    managed_token_id!(LOCKED_LP_TOKEN_ID)
                );
                assert_eq!(locked_tokens.token_nonce, 1);
                assert_eq!(locked_tokens.amount, managed_biguint!(1_000_000_000));

                assert_eq!(
                    reward_tokens.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(reward_tokens.token_nonce, 0);
                assert_eq!(
                    reward_tokens.amount,
                    managed_biguint!(15 * PER_BLOCK_REWARD_AMOUNT)
                );
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_LP_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000_000),
        Some(&lp_proxy_token_attributes),
    );
    b_mock.check_esdt_balance(
        &user_addr,
        MEX_TOKEN_ID,
        &rust_biguint!(25 * PER_BLOCK_REWARD_AMOUNT),
    );

    // user enter farm again
    b_mock.set_block_epoch(0);

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            LOCKED_LP_TOKEN_ID,
            1,
            &rust_biguint!(500_000_000),
            |sc| {
                let enter_farm_result = sc.enter_farm_locked_token(FarmType::SimpleFarm);
                let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_PROXY_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, 3);
                assert_eq!(out_farm_token.amount, managed_biguint!(500_000_000));
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        3,
        &rust_biguint!(500_000_000),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 3,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );

    // user enter farm along with previous position
    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(300_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARM_PROXY_TOKEN_ID.to_vec(),
            nonce: 3,
            value: rust_biguint!(500_000_000),
        },
    ];
    b_mock
        .execute_esdt_multi_transfer(&user_addr, &lock_wrapper, &payments, |sc| {
            let enter_farm_result = sc.enter_farm_locked_token(FarmType::SimpleFarm);
            let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
            assert_eq!(
                out_farm_token.token_identifier,
                managed_token_id!(FARM_PROXY_TOKEN_ID)
            );
            assert_eq!(out_farm_token.token_nonce, 4);
            assert_eq!(out_farm_token.amount, managed_biguint!(800_000_000));
        })
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        4,
        &rust_biguint!(800_000_000),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 4,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );

    // test enter with three additional farm tokens
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            LOCKED_LP_TOKEN_ID,
            1,
            &rust_biguint!(50_000_000),
            |sc| {
                sc.enter_farm_locked_token(FarmType::SimpleFarm);
            },
        )
        .assert_ok();
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            LOCKED_LP_TOKEN_ID,
            1,
            &rust_biguint!(50_000_000),
            |sc| {
                sc.enter_farm_locked_token(FarmType::SimpleFarm);
            },
        )
        .assert_ok();

    let payments = [
        TxTokenTransfer {
            token_identifier: LOCKED_LP_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(100_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARM_PROXY_TOKEN_ID.to_vec(),
            nonce: 4,
            value: rust_biguint!(800_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARM_PROXY_TOKEN_ID.to_vec(),
            nonce: 5,
            value: rust_biguint!(50_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARM_PROXY_TOKEN_ID.to_vec(),
            nonce: 6,
            value: rust_biguint!(50_000_000),
        },
    ];
    b_mock
        .execute_esdt_multi_transfer(&user_addr, &lock_wrapper, &payments, |sc| {
            let enter_farm_result = sc.enter_farm_locked_token(FarmType::SimpleFarm);
            let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
            assert_eq!(
                out_farm_token.token_identifier,
                managed_token_id!(FARM_PROXY_TOKEN_ID)
            );
            assert_eq!(out_farm_token.token_nonce, 7);
            assert_eq!(out_farm_token.amount, managed_biguint!(1_000_000_000));
        })
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        7,
        &rust_biguint!(1_000_000_000),
        Some(&FarmProxyTokenAttributes::<DebugApi> {
            farm_type: FarmType::SimpleFarm,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 7,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        }),
    );

    // exit farm
    b_mock.set_block_epoch(25);
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &lock_wrapper,
            FARM_PROXY_TOKEN_ID,
            7,
            &rust_biguint!(1_000_000_000),
            |sc| {
                let exit_farm_result = sc.exit_farm_locked_token(managed_biguint!(1_000_000_000));
                let (locked_tokens, _reward_tokens) = exit_farm_result.into_tuple();

                assert_eq!(
                    locked_tokens.token_identifier,
                    managed_token_id!(LOCKED_LP_TOKEN_ID)
                );
                assert_eq!(locked_tokens.token_nonce, 1);
                assert_eq!(locked_tokens.amount, managed_biguint!(1_000_000_000));
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_LP_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000_000),
        Some(&lp_proxy_token_attributes),
    );
}
