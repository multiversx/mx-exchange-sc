#![allow(deprecated)]

mod proxy_dex_test_setup;

use multiversx_sc::{codec::Empty, types::EsdtLocalRole};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
};
use pair::config::ConfigModule;
use proxy_dex_test_setup::*;
use proxy_dex_xmex::{
    create_pair_foundation::{CreatePairFoundationModule, UnlockInfo},
    create_pair_user::{CreatePairUserModule, ISSUE_COST},
};
use router::Router;

#[ignore = "Can't issue token in mock yet"]
#[test]
fn create_pair_test() {
    let mut setup = ProxySetup::new(
        proxy_dex_xmex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        router::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let full_balance = rust_biguint!(USER_BALANCE);
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = 499_999_000;
    let owner = setup.owner.clone();

    setup
        .b_mock
        .set_egld_balance(&first_user, &rust_biguint!(ISSUE_COST));

    // user deposit
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WEGLD_TOKEN_ID,
            0,
            &other_token_amount,
            |sc| {
                sc.deposit_project_token(managed_biguint!(1));
            },
        )
        .assert_ok();

    // user create pair
    setup
        .b_mock
        .execute_tx(
            &first_user,
            &setup.proxy_wrapper,
            &rust_biguint!(ISSUE_COST),
            |sc| {
                sc.create_xmex_token_pair(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_buffer!(b"Name"),
                    managed_buffer!(b"NAME"),
                );
            },
        )
        .assert_ok();

    // owner deposit
    setup
        .b_mock
        .execute_esdt_transfer(
            &owner,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &locked_token_amount,
            |sc| {
                sc.deposit_xmex(managed_token_id!(WEGLD_TOKEN_ID));
            },
        )
        .assert_ok();

    // try deposit for wrong token
    setup
        .b_mock
        .execute_esdt_transfer(
            &owner,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &locked_token_amount,
            |sc| {
                sc.deposit_xmex(managed_token_id!(b"FAKE-123456"));
            },
        )
        .assert_user_error("Tokens not deposited");

    // clear token info
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            sc.clear_token_info(managed_token_id!(WEGLD_TOKEN_ID));
        })
        .assert_ok();

    // withdraw xmex
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            sc.withdraw_xmex(managed_token_id!(WEGLD_TOKEN_ID));
        })
        .assert_ok();

    setup
        .b_mock
        .check_esdt_balance(&first_user, WEGLD_TOKEN_ID, &full_balance);
    setup
        .b_mock
        .check_nft_balance::<Empty>(&owner, LOCKED_TOKEN_ID, 1, &full_balance, None);

    // owner has to remove the pair from router
    setup
        .b_mock
        .execute_tx(&owner, &setup.router_wrapper, &rust_biguint!(0), |sc| {
            sc.remove_pair(
                managed_token_id!(WEGLD_TOKEN_ID),
                managed_token_id!(MEX_TOKEN_ID),
            );
        })
        .assert_ok();

    // user deposit
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WEGLD_TOKEN_ID,
            0,
            &other_token_amount,
            |sc| {
                sc.deposit_project_token(managed_biguint!(1));
            },
        )
        .assert_ok();

    let pair_wrapper = setup
        .b_mock
        .prepare_deploy_from_sc(setup.router_wrapper.address_ref(), pair::contract_obj);
    setup
        .b_mock
        .execute_tx(
            &first_user,
            &setup.proxy_wrapper,
            &rust_biguint!(ISSUE_COST),
            |sc| {
                sc.create_xmex_token_pair(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_buffer!(b"Name"),
                    managed_buffer!(b"NAME"),
                );
                let stored_pair_addr = sc
                    .get_pair_address(managed_token_id!(WEGLD_TOKEN_ID))
                    .into_option()
                    .unwrap()
                    .to_address();

                assert_eq!(pair_wrapper.address_ref(), &stored_pair_addr);
            },
        )
        .assert_ok();

    // owner deposit
    setup
        .b_mock
        .execute_esdt_transfer(
            &owner,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &locked_token_amount,
            |sc| {
                sc.deposit_xmex(managed_token_id!(WEGLD_TOKEN_ID));
            },
        )
        .assert_ok();

    // add initial liq wrong token ID
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            sc.add_initial_liq_from_deposits(managed_token_id!(b"FAKE-123456"));
        })
        .assert_user_error("Tokens not deposited");

    // owner needs to manually resume pair
    setup
        .b_mock
        .execute_tx(&owner, &setup.router_wrapper, &rust_biguint!(0), |sc| {
            sc.resume(managed_address!(pair_wrapper.address_ref()));
        })
        .assert_ok();

    // simulate issue of LP token
    setup
        .b_mock
        .execute_tx(&owner, &pair_wrapper, &rust_biguint!(0), |sc| {
            let lp_token_id = managed_token_id!(LP_TOKEN_ID);
            sc.lp_token_identifier().set(&lp_token_id);
        })
        .assert_ok();

    let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
    setup
        .b_mock
        .set_esdt_local_roles(pair_wrapper.address_ref(), LP_TOKEN_ID, &lp_token_roles[..]);

    let mut wrapped_token_nonce = 0;

    // add initial liq WEGLD
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            wrapped_token_nonce =
                sc.add_initial_liq_from_deposits(managed_token_id!(WEGLD_TOKEN_ID));

            let unlock_info = sc.lp_unlock_info(wrapped_token_nonce).get();
            let expected_unlock_info = UnlockInfo {
                unlock_epoch: 1 + LP_LOCK_EPOCHS,
                amount: managed_biguint!(expected_lp_token_amount),
                original_depositor_address: managed_address!(&first_user),
            };
            assert_eq!(unlock_info, expected_unlock_info);
        })
        .assert_ok();

    setup.b_mock.set_block_epoch(5);

    // try unlock early
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            let _ = sc.remove_liq_created_pair(
                managed_address!(pair_wrapper.address_ref()),
                managed_biguint!(1),
                managed_biguint!(1),
                wrapped_token_nonce,
            );
        })
        .assert_user_error("May not unlock yet");

    setup.b_mock.set_block_epoch(11);

    setup.b_mock.check_nft_balance::<Empty>(
        setup.proxy_wrapper.address_ref(),
        WRAPPED_LP_TOKEN_ID,
        1,
        &rust_biguint!(expected_lp_token_amount),
        None,
    );

    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &other_token_amount),
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &owner,
        LOCKED_TOKEN_ID,
        1,
        &(&full_balance - &locked_token_amount),
        None,
    );

    // remove liq
    setup
        .b_mock
        .execute_tx(&owner, &setup.proxy_wrapper, &rust_biguint!(0), |sc| {
            let _ = sc.remove_liq_created_pair(
                managed_address!(pair_wrapper.address_ref()),
                managed_biguint!(1),
                managed_biguint!(1),
                wrapped_token_nonce,
            );
        })
        .assert_ok();

    // users lose some tokens between add and remove liq, taken as fees by the pair
    setup.b_mock.check_esdt_balance(
        &first_user,
        WEGLD_TOKEN_ID,
        &(&full_balance - &rust_biguint!(1_000)),
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &owner,
        LOCKED_TOKEN_ID,
        1,
        &(full_balance - rust_biguint!(2_000)),
        None,
    );
}
