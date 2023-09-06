#![allow(deprecated)]

mod router_setup;
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{
        Address, EgldOrEsdtTokenIdentifier, EsdtLocalRole, ManagedAddress, ManagedVec,
        MultiValueEncoded,
    },
};
use pair::{config::ConfigModule, Pair};
use pausable::{PausableModule, State};
use router::{
    enable_swap_by_user::EnableSwapByUserModule,
    factory::{FactoryModule, PairTokens},
    multi_pair_swap::SWAP_TOKENS_FIXED_INPUT_FUNC_NAME,
    Router,
};
use router_setup::*;

use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::BlockchainStateWrapper, whitebox_legacy::TxTokenTransfer, DebugApi,
};
use simple_lock::{
    locked_token::{LockedTokenAttributes, LockedTokenModule},
    SimpleLock,
};

#[test]
fn test_router_setup() {
    let _ = RouterSetup::new(router::contract_obj, pair::contract_obj);
}

#[test]
fn test_router_upgrade_pair() {
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let user = b_mock.create_user_account(&rust_zero);

    b_mock.set_esdt_balance(
        &user,
        CUSTOM_TOKEN_ID,
        &rust_biguint!(USER_CUSTOM_TOKEN_BALANCE),
    );
    b_mock.set_esdt_balance(&user, USDC_TOKEN_ID, &rust_biguint!(USER_USDC_BALANCE));

    let router_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        router::contract_obj,
        ROUTER_WASM_PATH,
    );

    let pair_template_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(router_wrapper.address_ref()),
        pair::contract_obj,
        PAIR_WASM_PATH,
    );

    // setup pair
    b_mock
        .execute_tx(&owner, &pair_template_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(CUSTOM_TOKEN_ID);
            let second_token_id = managed_token_id!(USDC_TOKEN_ID);
            let router_address = managed_address!(&Address::zero());
            let router_owner_address = managed_address!(&owner);

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                0,
                0,
                managed_address!(&user),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );
        })
        .assert_ok();

    let pair_wrapper =
        b_mock.prepare_deploy_from_sc(router_wrapper.address_ref(), pair::contract_obj);

    b_mock
        .execute_tx(&owner, &router_wrapper, &rust_zero, |sc| {
            sc.init(OptionalValue::Some(managed_address!(
                pair_template_wrapper.address_ref()
            )));
            sc.set_pair_creation_enabled(true);
        })
        .assert_ok();

    b_mock
        .execute_tx(&user, &router_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(CUSTOM_TOKEN_ID);
            let second_token_id = managed_token_id!(USDC_TOKEN_ID);
            let _new_pair_address = sc.create_pair_endpoint(
                first_token_id,
                second_token_id,
                managed_address!(&user),
                OptionalValue::None,
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );
        })
        .assert_ok();

    b_mock
        .execute_tx(&owner, &router_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(CUSTOM_TOKEN_ID);
            let second_token_id = managed_token_id!(USDC_TOKEN_ID);
            sc.upgrade_pair_endpoint(
                first_token_id,
                second_token_id,
                managed_address!(&user),
                300,
                50,
            );
        })
        .assert_ok();

    b_mock
        .execute_query(&pair_wrapper, |sc| {
            let inital_liquidity_adder = sc.initial_liquidity_adder().get().unwrap();
            assert_eq!(inital_liquidity_adder, managed_address!(&user))
        })
        .assert_ok();
}

#[test]
fn test_multi_pair_swap() {
    let mut router_setup = RouterSetup::new(router::contract_obj, pair::contract_obj);
    router_setup.migrate_pair_map();

    router_setup.add_liquidity();

    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        WEGLD_TOKEN_ID,
        &rust_biguint!(5_000_000_000),
    );
    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        MEX_TOKEN_ID,
        &rust_biguint!(5_000_000_000),
    );
    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        USDC_TOKEN_ID,
        &rust_biguint!(5_000_000_000),
    );

    let ops = vec![
        (
            router_setup.mex_pair_wrapper.address_ref().clone(),
            SWAP_TOKENS_FIXED_INPUT_FUNC_NAME,
            WEGLD_TOKEN_ID, //swap to wegld
            1,
        ),
        (
            router_setup.usdc_pair_wrapper.address_ref().clone(),
            SWAP_TOKENS_FIXED_INPUT_FUNC_NAME,
            USDC_TOKEN_ID, //swap to usdc
            1,
        ),
    ];

    router_setup.multi_pair_swap(MEX_TOKEN_ID, 100_000, &ops);

    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        WEGLD_TOKEN_ID,
        &rust_biguint!(5_000_000_000), //unchanged
    );
    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        MEX_TOKEN_ID,
        &rust_biguint!(4_999_900_000), //spent 100_000
    );
    router_setup.blockchain_wrapper.check_esdt_balance(
        &router_setup.user_address,
        USDC_TOKEN_ID,
        &rust_biguint!(5_000_082_909), //gained 82_909
    );
}

#[test]
fn user_enable_pair_swaps_through_router_test() {
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let user = b_mock.create_user_account(&rust_zero);

    let current_epoch = 5;
    b_mock.set_block_epoch(current_epoch);

    b_mock.set_esdt_balance(
        &user,
        CUSTOM_TOKEN_ID,
        &rust_biguint!(USER_CUSTOM_TOKEN_BALANCE),
    );
    b_mock.set_esdt_balance(&user, USDC_TOKEN_ID, &rust_biguint!(USER_USDC_BALANCE));

    let router_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        router::contract_obj,
        ROUTER_WASM_PATH,
    );
    let pair_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(router_wrapper.address_ref()),
        pair::contract_obj,
        PAIR_WASM_PATH,
    );
    let simple_lock_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        simple_lock::contract_obj,
        "simple-lock.wasm",
    );

    // setup simple-lock
    b_mock
        .execute_tx(&owner, &simple_lock_wrapper, &rust_zero, |sc| {
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        simple_lock_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // setup router
    b_mock
        .execute_tx(&owner, &router_wrapper, &rust_zero, |sc| {
            sc.init(OptionalValue::None);

            sc.pair_map().insert(
                PairTokens {
                    first_token_id: managed_token_id!(CUSTOM_TOKEN_ID),
                    second_token_id: managed_token_id!(USDC_TOKEN_ID),
                },
                managed_address!(pair_wrapper.address_ref()),
            );

            sc.migrate_pair_map();

            sc.add_common_tokens_for_user_pairs(MultiValueEncoded::from(ManagedVec::from(vec![
                managed_token_id!(USDC_TOKEN_ID),
            ])));

            sc.config_enable_by_user_parameters(
                managed_token_id!(USDC_TOKEN_ID),
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_biguint!(MIN_LOCKED_TOKEN_VALUE),
                MIN_LOCKED_PERIOD_EPOCHS,
            )
        })
        .assert_ok();

    // setup pair
    b_mock
        .execute_tx(&owner, &pair_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(CUSTOM_TOKEN_ID);
            let second_token_id = managed_token_id!(USDC_TOKEN_ID);
            let router_address = managed_address!(router_wrapper.address_ref());
            let router_owner_address = managed_address!(&owner);

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                0,
                0,
                managed_address!(&user),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );

            assert_eq!(sc.state().get(), State::Inactive);

            sc.lp_token_identifier()
                .set(&managed_token_id!(LPUSDC_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        pair_wrapper.address_ref(),
        LPUSDC_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );

    // add liquidity
    let payments = vec![
        TxTokenTransfer {
            token_identifier: CUSTOM_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(USER_CUSTOM_TOKEN_BALANCE),
        },
        TxTokenTransfer {
            token_identifier: USDC_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(USER_USDC_BALANCE),
        },
    ];

    let user_lp_tokens_balance = 999_000u64;
    b_mock
        .execute_esdt_multi_transfer(&user, &pair_wrapper, &payments, |sc| {
            let (lp_tokens_received, _, _) = sc.add_initial_liquidity().into_tuple();
            assert_eq!(
                lp_tokens_received.token_identifier,
                managed_token_id!(LPUSDC_TOKEN_ID)
            );
            assert_eq!(
                lp_tokens_received.amount,
                managed_biguint!(user_lp_tokens_balance)
            );
        })
        .assert_ok();

    // lock LP tokens
    b_mock
        .execute_esdt_transfer(
            &user,
            &simple_lock_wrapper,
            LPUSDC_TOKEN_ID,
            0,
            &rust_biguint!(user_lp_tokens_balance),
            |sc| {
                sc.lock_tokens_endpoint(
                    current_epoch + MIN_LOCKED_PERIOD_EPOCHS,
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    b_mock.check_nft_balance(
        &user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(user_lp_tokens_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(LPUSDC_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: current_epoch + MIN_LOCKED_PERIOD_EPOCHS,
        }),
    );

    // pass blocks time to update safe price
    b_mock.set_block_nonce(1_000_000);

    // activate swaps through router
    b_mock
        .execute_esdt_transfer(
            &user,
            &router_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(user_lp_tokens_balance),
            |sc| {
                sc.set_swap_enabled_by_user(managed_address!(pair_wrapper.address_ref()));
            },
        )
        .assert_ok();

    // check pair state is active
    b_mock
        .execute_query(&pair_wrapper, |sc| {
            assert_eq!(sc.state().get(), State::Active);
        })
        .assert_ok();

    // check user received the locked tokens back
    b_mock.check_nft_balance(
        &user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(user_lp_tokens_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(LPUSDC_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: current_epoch + MIN_LOCKED_PERIOD_EPOCHS,
        }),
    );
}

#[test]
fn user_enable_pair_swaps_fail_test() {
    let rust_zero = rust_biguint!(0u64);
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let user = b_mock.create_user_account(&rust_zero);

    let current_epoch = 5;
    b_mock.set_block_epoch(current_epoch);

    b_mock.set_esdt_balance(
        &user,
        CUSTOM_TOKEN_ID,
        &rust_biguint!(USER_CUSTOM_TOKEN_BALANCE),
    );
    b_mock.set_esdt_balance(&user, USDC_TOKEN_ID, &rust_biguint!(USER_USDC_BALANCE));

    let router_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        router::contract_obj,
        ROUTER_WASM_PATH,
    );
    let pair_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(router_wrapper.address_ref()),
        pair::contract_obj,
        PAIR_WASM_PATH,
    );

    // setup router
    b_mock
        .execute_tx(&owner, &router_wrapper, &rust_zero, |sc| {
            sc.init(OptionalValue::None);

            sc.pair_map().insert(
                PairTokens {
                    first_token_id: managed_token_id!(CUSTOM_TOKEN_ID),
                    second_token_id: managed_token_id!(USDC_TOKEN_ID),
                },
                managed_address!(pair_wrapper.address_ref()),
            );

            sc.migrate_pair_map();

            sc.add_common_tokens_for_user_pairs(MultiValueEncoded::from(ManagedVec::from(vec![
                managed_token_id!(USDC_TOKEN_ID),
            ])));

            sc.config_enable_by_user_parameters(
                managed_token_id!(USDC_TOKEN_ID),
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_biguint!(MIN_LOCKED_TOKEN_VALUE),
                MIN_LOCKED_PERIOD_EPOCHS,
            )
        })
        .assert_ok();

    // setup pair
    b_mock
        .execute_tx(&owner, &pair_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(CUSTOM_TOKEN_ID);
            let second_token_id = managed_token_id!(USDC_TOKEN_ID);
            let router_address = managed_address!(router_wrapper.address_ref());
            let router_owner_address = managed_address!(&owner);

            sc.init(
                first_token_id,
                second_token_id,
                router_address,
                router_owner_address,
                0,
                0,
                managed_address!(&user),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
            );

            assert_eq!(sc.state().get(), State::Inactive);

            sc.lp_token_identifier()
                .set(&managed_token_id!(LPUSDC_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        pair_wrapper.address_ref(),
        LPUSDC_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );

    // add liquidity
    let payments = vec![
        TxTokenTransfer {
            token_identifier: CUSTOM_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(USER_CUSTOM_TOKEN_BALANCE),
        },
        TxTokenTransfer {
            token_identifier: USDC_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(USER_USDC_BALANCE),
        },
    ];

    let user_lp_tokens_balance = 999_000u64;
    b_mock
        .execute_esdt_multi_transfer(&user, &pair_wrapper, &payments, |sc| {
            let (lp_tokens_received, _, _) = sc.add_initial_liquidity().into_tuple();
            assert_eq!(
                lp_tokens_received.token_identifier,
                managed_token_id!(LPUSDC_TOKEN_ID)
            );
            assert_eq!(
                lp_tokens_received.amount,
                managed_biguint!(user_lp_tokens_balance)
            );
        })
        .assert_ok();

    let custom_locked_token = b"LTOK2-123456";
    let _ = DebugApi::dummy();
    b_mock.set_nft_balance(
        &user,
        custom_locked_token,
        1,
        &rust_biguint!(user_lp_tokens_balance),
        &LockedTokenAttributes::<DebugApi> {
            original_token_id: EgldOrEsdtTokenIdentifier::esdt(managed_token_id!(LPUSDC_TOKEN_ID)),
            original_token_nonce: 0,
            unlock_epoch: current_epoch + MIN_LOCKED_PERIOD_EPOCHS,
        },
    );

    // pass blocks time to update safe price
    b_mock.set_block_nonce(1_000_000);

    // activate swaps through router
    b_mock
        .execute_esdt_transfer(
            &user,
            &router_wrapper,
            custom_locked_token,
            1,
            &rust_biguint!(user_lp_tokens_balance),
            |sc| {
                sc.set_swap_enabled_by_user(managed_address!(pair_wrapper.address_ref()));
            },
        )
        .assert_user_error("Invalid locked token");

    // check pair state is active
    b_mock
        .execute_query(&pair_wrapper, |sc| {
            assert_eq!(sc.state().get(), State::PartialActive);
        })
        .assert_ok();

    // check user received the locked tokens back
    b_mock.check_nft_balance(
        &user,
        custom_locked_token,
        1,
        &rust_biguint!(user_lp_tokens_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(LPUSDC_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: current_epoch + MIN_LOCKED_PERIOD_EPOCHS,
        }),
    );
}
