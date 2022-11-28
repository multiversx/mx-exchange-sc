mod pair_setup;
use elrond_wasm::{
    elrond_codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{EsdtLocalRole, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    tx_mock::TxInputESDT, DebugApi,
};
use fees_collector::{
    config::ConfigModule, fees_accumulation::FeesAccumulationModule, FeesCollector,
};
use pair::{config::MAX_PERCENTAGE, fee::FeeModule, locking_wrapper::LockingWrapperModule, Pair};
use pair_setup::*;
use simple_lock::{
    locked_token::{LockedTokenAttributes, LockedTokenModule},
    proxy_lp::{LpProxyTokenAttributes, ProxyLpModule},
    SimpleLock,
};

#[test]
fn test_pair_setup() {
    let _ = PairSetup::new(pair::contract_obj);
}

#[test]
fn test_add_liquidity() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );
}

#[test]
fn test_swap_fixed_input() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 996);
}

#[test]
fn test_swap_fixed_output() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_output(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 96);
}

#[test]
fn test_safe_price() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.b_mock.set_block_nonce(11);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 996);
    pair_setup.check_current_safe_state(11, 11, 1, 1_001_000, 1_001_000, 1_001_000, 1_001_000);
    pair_setup.check_future_safe_state(0 /* for rust format */, 0, 0, 0, 0, 0, 0);

    pair_setup.b_mock.set_block_nonce(20);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 994);
    pair_setup.check_current_safe_state(11, 20, 2, 1_002_000, 1_000_004, 1_001_000, 1_001_000);
    pair_setup.check_future_safe_state(0 /* for rust format */, 0, 0, 0, 0, 0, 0);

    pair_setup.b_mock.set_block_nonce(30);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 992);
    pair_setup.check_current_safe_state(11, 30, 3, 1_003_000, 999_010, 1_001_500, 1_000_502);
    pair_setup.check_future_safe_state(0 /* for rust format */, 0, 0, 0, 0, 0, 0);

    pair_setup.b_mock.set_block_nonce(40);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 990);
    pair_setup.check_current_safe_state(11, 40, 4, 1_004_000, 998_018, 1_002_000, 1_000_004);
    pair_setup.check_future_safe_state(0 /* for rust format */, 0, 0, 0, 0, 0, 0);

    pair_setup.b_mock.set_block_nonce(50);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 988);
    pair_setup.check_current_safe_state(11, 50, 5, 1_005_000, 997_028, 1_002_500, 999_507);
    pair_setup.check_future_safe_state(0 /* for rust format */, 0, 0, 0, 0, 0, 0);

    pair_setup.b_mock.set_block_nonce(60);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 986);
    pair_setup.check_current_safe_state(11, 60, 6, 1_006_000, 996_040, 1_003_000, 999_011);
    pair_setup.check_future_safe_state(60, 60, 1, 1_006_000, 996_040, 1_006_000, 996_040);

    pair_setup.b_mock.set_block_nonce(70);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 984);
    pair_setup.check_current_safe_state(11, 70, 7, 1_007_000, 995_054, 1_003_500, 998_515);
    pair_setup.check_future_safe_state(60, 70, 2, 1_007_000, 995_054, 1_006_000, 996_040);

    pair_setup.b_mock.set_block_nonce(80);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 982);

    pair_setup.b_mock.set_block_nonce(90);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 980);

    pair_setup.b_mock.set_block_nonce(100);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 978);
    pair_setup.check_current_safe_state(11, 100, 10, 1_010_000, 992_108, 1_005_000, 997_032);
    pair_setup.check_future_safe_state(60, 100, 5, 1_010_000, 992_108, 1_007_462, 994_598);

    pair_setup.b_mock.set_block_nonce(110);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 900, 976);
    pair_setup.check_current_safe_state(60, 110, 6, 1_011_000, 991_130, 1_007_959, 994_109);
    pair_setup.check_future_safe_state(110, 110, 1, 1_011_000, 991_130, 1_011_000, 991_130);
}

#[test]
fn test_swap_protect() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    let protect_until_block = 10;
    let max_volume_percent = 10_000;
    let max_num_swaps = 2;
    pair_setup.set_swap_protect(protect_until_block, max_volume_percent, max_num_swaps);

    pair_setup.swap_fixed_input_expect_error(
        WEGLD_TOKEN_ID,
        500_000,
        MEX_TOKEN_ID,
        1,
        "swap amount in too large",
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 1, 996);
    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 1_000, MEX_TOKEN_ID, 1, 994);

    pair_setup.swap_fixed_input_expect_error(
        WEGLD_TOKEN_ID,
        1_000,
        MEX_TOKEN_ID,
        1,
        "too many swaps by address",
    );

    pair_setup.b_mock.set_block_nonce(protect_until_block + 1);

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 500_000, MEX_TOKEN_ID, 1, 331_672);
}

#[test]
fn test_locked_asset() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // init locking SC
    let rust_zero = rust_biguint!(0);
    let locking_owner = pair_setup.b_mock.create_user_account(&rust_zero);
    let locking_sc_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_zero,
        Some(&locking_owner),
        simple_lock::contract_obj,
        "Some path",
    );

    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    pair_setup.b_mock.set_block_epoch(4);

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_locking_sc_address(managed_address!(locking_sc_wrapper.address_ref()));
                sc.set_locking_deadline_epoch(5);
                sc.set_unlock_epoch(10);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &pair_setup.pair_wrapper,
            &MEX_TOKEN_ID,
            0,
            &rust_biguint!(1_000),
            |sc| {
                let ret = sc.swap_tokens_fixed_input(
                    managed_token_id!(WEGLD_TOKEN_ID),
                    managed_biguint!(10),
                );

                assert_eq!(ret.token_identifier, managed_token_id!(LOCKED_TOKEN_ID));
                assert_eq!(ret.token_nonce, 1);
                assert_eq!(ret.amount, managed_biguint!(996));
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(996),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );

    let user_wegld_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, WEGLD_TOKEN_ID, 0);

    // try unlock too early
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(996),
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_user_error("Cannot unlock yet");

    // unlock ok
    pair_setup.b_mock.set_block_epoch(20);

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(996),
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        WEGLD_TOKEN_ID,
        &(user_wegld_balance_before + rust_biguint!(996)),
    );
}

#[test]
fn add_liquidity_through_simple_lock_proxy() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    // init locking SC
    let lp_address = pair_setup.pair_wrapper.address_ref().clone();
    let rust_zero = rust_biguint!(0);
    let locking_owner = pair_setup.b_mock.create_user_account(&rust_zero);
    let locking_sc_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_zero,
        Some(&locking_owner),
        simple_lock::contract_obj,
        "Some path",
    );

    // setup locked token
    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.add_lp_to_whitelist(
                managed_address!(&lp_address),
                managed_token_id!(WEGLD_TOKEN_ID),
                managed_token_id!(MEX_TOKEN_ID),
            );
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // setup lp proxy token
    pair_setup
        .b_mock
        .execute_tx(&locking_owner, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LP_PROXY_TOKEN_ID));
        })
        .assert_ok();

    pair_setup.b_mock.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    pair_setup.b_mock.set_block_epoch(5);
    let _ = DebugApi::dummy();

    // lock some tokens first
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            WEGLD_TOKEN_ID,
            0,
            &rust_biguint!(1_000_000),
            |sc| {
                sc.lock_tokens_endpoint(10, OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1_000_000),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(2_000_000),
            |sc| {
                sc.lock_tokens_endpoint(15, OptionalValue::None);
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(2_000_000),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 15,
        }),
    );

    pair_setup.b_mock.set_block_epoch(5);

    // add liquidity through simple-lock SC - one locked (WEGLD) token, one unlocked (MEX)
    let transfers = vec![
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(500_000),
        },
        TxInputESDT {
            token_identifier: MEX_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(500_000),
        },
    ];

    pair_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            &transfers[..],
            |sc| {
                let (dust_first_token, dust_second_token, lp_proxy_payment) = sc
                    .add_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    dust_first_token.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(dust_first_token.token_nonce, 0);
                assert_eq!(dust_first_token.amount, managed_biguint!(0));

                assert_eq!(
                    dust_second_token.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(dust_second_token.token_nonce, 0);
                assert_eq!(dust_second_token.amount, managed_biguint!(0));

                assert_eq!(
                    lp_proxy_payment.token_identifier,
                    managed_token_id!(LP_PROXY_TOKEN_ID)
                );
                assert_eq!(lp_proxy_payment.token_nonce, 1);
                assert_eq!(lp_proxy_payment.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();
    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LP_PROXY_TOKEN_ID,
        1,
        &rust_biguint!(500_000),
        Some(&LpProxyTokenAttributes::<DebugApi> {
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
            first_token_locked_nonce: 1,
            second_token_id: managed_token_id!(MEX_TOKEN_ID),
            second_token_locked_nonce: 0,
        }),
    );
    pair_setup.b_mock.check_esdt_balance(
        locking_sc_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(500_000),
    );

    let user_locked_token_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, LOCKED_TOKEN_ID, 1);
    let user_mex_balance_before =
        pair_setup
            .b_mock
            .get_esdt_balance(&pair_setup.user_address, MEX_TOKEN_ID, 0);

    // remove liquidity
    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LP_PROXY_TOKEN_ID,
            1,
            &rust_biguint!(500_000),
            |sc| {
                let (first_payment_result, second_payment_result) = sc
                    .remove_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    first_payment_result.token_identifier,
                    managed_token_id!(LOCKED_TOKEN_ID)
                );
                assert_eq!(first_payment_result.token_nonce, 1);
                assert_eq!(first_payment_result.amount, managed_biguint!(500_000));

                assert_eq!(
                    second_payment_result.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(second_payment_result.token_nonce, 0);
                assert_eq!(second_payment_result.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    pair_setup.b_mock.check_nft_balance(
        &pair_setup.user_address,
        LOCKED_TOKEN_ID,
        1,
        &(user_locked_token_balance_before + 500_000u32),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(WEGLD_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 10,
        }),
    );
    pair_setup.b_mock.check_esdt_balance(
        &pair_setup.user_address,
        MEX_TOKEN_ID,
        &(user_mex_balance_before + 500_000u32),
    );

    // Add liquidity - same token pair as before -> same nonce (1)
    pair_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            &transfers[..],
            |sc| {
                let (_, _, lp_proxy_payment) = sc
                    .add_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    lp_proxy_payment.token_identifier,
                    managed_token_id!(LP_PROXY_TOKEN_ID)
                );
                assert_eq!(lp_proxy_payment.token_nonce, 1);
                assert_eq!(lp_proxy_payment.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    // test auto-unlock for tokens on remove liquidity
    pair_setup.b_mock.set_block_epoch(30);

    pair_setup
        .b_mock
        .execute_esdt_transfer(
            &pair_setup.user_address,
            &locking_sc_wrapper,
            LP_PROXY_TOKEN_ID,
            1,
            &rust_biguint!(500_000),
            |sc| {
                let (first_payment_result, second_payment_result) = sc
                    .remove_liquidity_locked_token(managed_biguint!(1), managed_biguint!(1))
                    .into_tuple();

                assert_eq!(
                    first_payment_result.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(first_payment_result.token_nonce, 0);
                assert_eq!(first_payment_result.amount, managed_biguint!(500_000));

                assert_eq!(
                    second_payment_result.token_identifier,
                    managed_token_id!(MEX_TOKEN_ID)
                );
                assert_eq!(second_payment_result.token_nonce, 0);
                assert_eq!(second_payment_result.amount, managed_biguint!(500_000));
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_query(&locking_sc_wrapper, |sc| {
            assert_eq!(sc.known_liquidity_pools().len(), 1);
            assert_eq!(
                sc.known_liquidity_pools()
                    .contains(&managed_address!(&lp_address)),
                true
            );
        })
        .assert_ok();
}

#[test]
fn fees_collector_pair_test() {
    let mut pair_setup = PairSetup::new(pair::contract_obj);
    let fees_collector_wrapper = pair_setup.b_mock.create_sc_account(
        &rust_biguint!(0),
        None,
        fees_collector::contract_obj,
        "fees collector path",
    );

    let pair_addr = pair_setup.pair_wrapper.address_ref().clone();
    let energy_factory_mock_addr = pair_setup.pair_wrapper.address_ref().clone();
    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &fees_collector_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.init(
                    managed_token_id!(LOCKED_TOKEN_ID),
                    managed_address!(&energy_factory_mock_addr),
                );
                let _ = sc.known_contracts().insert(managed_address!(&pair_addr));

                let mut tokens = MultiValueEncoded::new();
                tokens.push(managed_token_id!(WEGLD_TOKEN_ID));
                tokens.push(managed_token_id!(MEX_TOKEN_ID));

                sc.add_known_tokens(tokens);
            },
        )
        .assert_ok();

    pair_setup
        .b_mock
        .execute_tx(
            &pair_setup.owner_address,
            &pair_setup.pair_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.setup_fees_collector(
                    managed_address!(fees_collector_wrapper.address_ref()),
                    MAX_PERCENTAGE / 2,
                );
            },
        )
        .assert_ok();

    pair_setup.add_liquidity(
        1_001_000, 1_000_000, 1_001_000, 1_000_000, 1_000_000, 1_001_000, 1_001_000,
    );

    pair_setup.swap_fixed_input(WEGLD_TOKEN_ID, 100_000, MEX_TOKEN_ID, 900, 90_669);

    pair_setup.b_mock.check_esdt_balance(
        fees_collector_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(25),
    );

    pair_setup
        .b_mock
        .execute_query(&fees_collector_wrapper, |sc| {
            assert_eq!(
                sc.accumulated_fees(1, &managed_token_id!(WEGLD_TOKEN_ID))
                    .get(),
                managed_biguint!(25)
            );
        })
        .assert_ok();
}
