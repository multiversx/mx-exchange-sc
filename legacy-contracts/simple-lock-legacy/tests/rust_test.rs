#![allow(deprecated)]

use common_structs::FarmTokenAttributes;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::imports::ContractBase;
use multiversx_sc::types::EsdtLocalRole;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, DebugApi,
};
use multiversx_sc_scenario::{managed_token_id_wrapped, whitebox_legacy::*};

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use simple_lock_legacy::proxy_farm::{FarmProxyTokenAttributes, FarmType, ProxyFarmModule};
use simple_lock_legacy::proxy_lp::{LpProxyTokenAttributes, ProxyLpModule};
use simple_lock_legacy::{locked_token::*, SimpleLockLegacy};

const FREE_TOKEN_ID: &[u8] = b"FREEEEE-123456";
const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";
const LOCKED_TOKEN_ID: &[u8] = b"NOOO0-123456";
const LP_TOKEN_ID: &[u8] = b"LPTOK-123456";
const LP_PROXY_TOKEN_ID: &[u8] = b"LPPROXY-123456";
const FARM_TOKEN_ID: &[u8] = b"FARMTOK-123456";
const FARM_PROXY_TOKEN_ID: &[u8] = b"FARMPROXY-123456";

#[test]
fn unlock_token_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        simple_lock_legacy::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LP_PROXY_TOKEN_ID));
            sc.farm_proxy_token()
                .set_token_id(managed_token_id!(FARM_PROXY_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    let lock_token_nonce = 1u64;
    let lock_amount = rust_biguint!(1_000);

    DebugApi::dummy();
    b_mock.set_esdt_balance(sc_wrapper.address_ref(), FREE_TOKEN_ID, &lock_amount);
    let locked_token_attributes: LockedTokenAttributes<DebugApi> = LockedTokenAttributes {
        original_token_id: managed_token_id_wrapped!(FREE_TOKEN_ID),
        original_token_nonce: 0,
        unlock_epoch: 10u64,
    };
    b_mock.set_nft_balance(
        &user_addr,
        LOCKED_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        &locked_token_attributes,
    );

    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        Some(&locked_token_attributes),
    );
    b_mock.check_esdt_balance(sc_wrapper.address_ref(), FREE_TOKEN_ID, &lock_amount);
    b_mock.check_esdt_balance(&user_addr, FREE_TOKEN_ID, &rust_zero);

    // unlock ok
    b_mock.set_block_epoch(10);
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LOCKED_TOKEN_ID,
            lock_token_nonce,
            &lock_amount,
            |sc| {
                sc.unlock_tokens_endpoint(OptionalValue::None);
            },
        )
        .assert_ok();
    b_mock.check_esdt_balance(&user_addr, FREE_TOKEN_ID, &lock_amount);
    b_mock.check_nft_balance(
        &user_addr,
        LOCKED_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        Some(&locked_token_attributes),
    );
    b_mock.check_esdt_balance(sc_wrapper.address_ref(), FREE_TOKEN_ID, &rust_zero);
    b_mock.check_nft_balance(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        Some(&locked_token_attributes),
    );
}

#[test]
fn exit_lp_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        simple_lock_legacy::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LP_PROXY_TOKEN_ID));
            sc.farm_proxy_token()
                .set_token_id(managed_token_id!(FARM_PROXY_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    let lock_token_nonce = 1u64;
    let lock_amount = rust_biguint!(1_000);

    DebugApi::dummy();
    // Simulate the user add liquidity step by setting the SC balance manually
    b_mock.set_esdt_balance(sc_wrapper.address_ref(), LP_TOKEN_ID, &lock_amount);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            let lp_proxy_token_amount = managed_biguint!(1_000u64);
            // Needed to be able to read the token attributes
            sc.send().esdt_nft_create_compact(
                &managed_token_id!(LOCKED_TOKEN_ID),
                &managed_biguint!(1u64),
                &LockedTokenAttributes::<DebugApi> {
                    original_token_id: managed_token_id_wrapped!(FREE_TOKEN_ID),
                    original_token_nonce: 0,
                    unlock_epoch: 10,
                },
            );
            let lp_proxy_token_nonce = sc.send().esdt_nft_create_compact(
                &managed_token_id!(LP_PROXY_TOKEN_ID),
                &lp_proxy_token_amount,
                &LpProxyTokenAttributes::<DebugApi> {
                    lp_token_id: managed_token_id!(LP_TOKEN_ID),
                    first_token_id: managed_token_id!(FREE_TOKEN_ID),
                    first_token_locked_nonce: 1,
                    second_token_id: managed_token_id!(WEGLD_TOKEN_ID),
                    second_token_locked_nonce: 0,
                },
            );

            sc.send().direct_esdt(
                &managed_address!(&user_addr),
                &managed_token_id!(LP_PROXY_TOKEN_ID),
                lp_proxy_token_nonce,
                &lp_proxy_token_amount,
            );
        })
        .assert_ok();

    let locked_lp_token_attributes: LpProxyTokenAttributes<DebugApi> = LpProxyTokenAttributes {
        lp_token_id: managed_token_id!(LP_TOKEN_ID),
        first_token_id: managed_token_id!(FREE_TOKEN_ID),
        first_token_locked_nonce: 1,
        second_token_id: managed_token_id!(WEGLD_TOKEN_ID),
        second_token_locked_nonce: 0,
    };
    b_mock.check_nft_balance(
        &user_addr,
        LP_PROXY_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        Some(&locked_lp_token_attributes),
    );

    // unlock ok
    b_mock.set_block_epoch(10);
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            LP_PROXY_TOKEN_ID,
            lock_token_nonce,
            &lock_amount,
            |sc| {
                sc.remove_liquidity_locked_token(managed_biguint!(0u64), managed_biguint!(0u64));
            },
        )
        .assert_ok();
    b_mock.check_esdt_balance(&user_addr, LP_TOKEN_ID, &lock_amount);
    b_mock.check_esdt_balance(sc_wrapper.address_ref(), LP_TOKEN_ID, &rust_zero);
    b_mock.check_nft_balance(
        &user_addr,
        LP_PROXY_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        Some(&locked_lp_token_attributes),
    );
    b_mock.check_nft_balance(
        sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        Some(&locked_lp_token_attributes),
    );
}

#[test]
fn exit_farm_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        simple_lock_legacy::contract_obj,
        "Some path",
    );

    b_mock.set_block_epoch(5);

    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.lp_proxy_token()
                .set_token_id(managed_token_id!(LP_PROXY_TOKEN_ID));
            sc.farm_proxy_token()
                .set_token_id(managed_token_id!(FARM_PROXY_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        LP_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );
    b_mock.set_esdt_local_roles(
        sc_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    let lock_token_nonce = 1u64;
    let lock_amount = rust_biguint!(2_000u64);

    DebugApi::dummy();
    b_mock
        .execute_tx(&owner_addr, &sc_wrapper, &rust_zero, |sc| {
            let farm_proxy_token_amount = managed_biguint!(2_000u64);
            // Needed to be able to read the token attributes
            sc.send().esdt_nft_create_compact(
                &managed_token_id!(LOCKED_TOKEN_ID),
                &managed_biguint!(1u64),
                &LockedTokenAttributes::<DebugApi> {
                    original_token_id: managed_token_id_wrapped!(FREE_TOKEN_ID),
                    original_token_nonce: 0,
                    unlock_epoch: 10,
                },
            );
            sc.send().esdt_nft_create_compact(
                &managed_token_id!(LP_PROXY_TOKEN_ID),
                &farm_proxy_token_amount,
                &LpProxyTokenAttributes::<DebugApi> {
                    lp_token_id: managed_token_id!(LP_TOKEN_ID),
                    first_token_id: managed_token_id!(FREE_TOKEN_ID),
                    first_token_locked_nonce: 1,
                    second_token_id: managed_token_id!(WEGLD_TOKEN_ID),
                    second_token_locked_nonce: 0,
                },
            );
            let farm_proxy_token_nonce = sc.send().esdt_nft_create_compact(
                &managed_token_id!(FARM_PROXY_TOKEN_ID),
                &farm_proxy_token_amount,
                &FarmProxyTokenAttributes::<DebugApi> {
                    farm_type: FarmType::FarmWithLockedRewards,
                    farm_token_id: managed_token_id!(FARM_TOKEN_ID),
                    farm_token_nonce: 1,
                    farming_token_id: managed_token_id!(LP_TOKEN_ID),
                    farming_token_locked_nonce: 1,
                },
            );

            sc.send().direct_esdt(
                &managed_address!(&user_addr),
                &managed_token_id!(FARM_PROXY_TOKEN_ID),
                farm_proxy_token_nonce,
                &farm_proxy_token_amount,
            );
        })
        .assert_ok();

    let locked_farm_token_attributes: FarmProxyTokenAttributes<DebugApi> =
        FarmProxyTokenAttributes {
            farm_type: FarmType::FarmWithLockedRewards,
            farm_token_id: managed_token_id!(FARM_TOKEN_ID),
            farm_token_nonce: 1,
            farming_token_id: managed_token_id!(LP_TOKEN_ID),
            farming_token_locked_nonce: 1,
        };
    b_mock.check_nft_balance(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        Some(&locked_farm_token_attributes),
    );

    let farm_attributes: FarmTokenAttributes<DebugApi> = FarmTokenAttributes {
        reward_per_share: managed_biguint!(0u64),
        entering_epoch: 5u64,
        compounded_reward: managed_biguint!(0u64),
        current_farm_amount: managed_biguint!(2_000u64),
        original_owner: managed_address!(&user_addr),
    };
    b_mock.set_nft_balance(
        sc_wrapper.address_ref(),
        FARM_TOKEN_ID,
        1,
        &lock_amount,
        &farm_attributes,
    );
    b_mock.check_nft_balance::<FarmTokenAttributes<DebugApi>>(
        &user_addr,
        FARM_TOKEN_ID,
        1,
        &rust_zero,
        None,
    );
    b_mock.check_nft_balance(
        sc_wrapper.address_ref(),
        FARM_TOKEN_ID,
        1,
        &lock_amount,
        Some(&farm_attributes),
    );

    // unlock ok
    let half_lock_amount = rust_biguint!(1_000u64);
    b_mock.set_block_epoch(10);
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FARM_PROXY_TOKEN_ID,
            lock_token_nonce,
            &half_lock_amount,
            |sc| {
                sc.farm_claim_rewards_locked_token();
            },
        )
        .assert_ok();

    b_mock.check_nft_balance::<FarmTokenAttributes<DebugApi>>(
        sc_wrapper.address_ref(),
        FARM_TOKEN_ID,
        lock_token_nonce,
        &half_lock_amount,
        Some(&farm_attributes),
    );
    b_mock.check_nft_balance(
        &user_addr,
        FARM_TOKEN_ID,
        lock_token_nonce,
        &half_lock_amount,
        Some(&farm_attributes),
    );
    b_mock.check_nft_balance::<FarmProxyTokenAttributes<DebugApi>>(
        sc_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        None,
    );
    b_mock.check_nft_balance::<FarmProxyTokenAttributes<DebugApi>>(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        lock_token_nonce,
        &half_lock_amount,
        Some(&locked_farm_token_attributes),
    );

    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &sc_wrapper,
            FARM_PROXY_TOKEN_ID,
            lock_token_nonce,
            &half_lock_amount,
            |sc| {
                sc.exit_farm_locked_token();
            },
        )
        .assert_ok();

    b_mock.check_nft_balance::<FarmTokenAttributes<DebugApi>>(
        sc_wrapper.address_ref(),
        FARM_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        None,
    );
    b_mock.check_nft_balance(
        &user_addr,
        FARM_TOKEN_ID,
        lock_token_nonce,
        &lock_amount,
        Some(&farm_attributes),
    );
    b_mock.check_nft_balance::<FarmProxyTokenAttributes<DebugApi>>(
        sc_wrapper.address_ref(),
        FARM_PROXY_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        None,
    );
    b_mock.check_nft_balance::<FarmProxyTokenAttributes<DebugApi>>(
        &user_addr,
        FARM_PROXY_TOKEN_ID,
        lock_token_nonce,
        &rust_zero,
        None,
    );
}
