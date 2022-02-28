use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm::types::{Address, EsdtLocalRole, ManagedAddress};
use elrond_wasm_debug::tx_mock::TxResult;
use elrond_wasm_debug::{managed_biguint, testing_framework::*};
use elrond_wasm_debug::{managed_token_id, rust_biguint, DebugApi};
use num_traits::ToPrimitive;

use price_discovery::create_pool::*;
use price_discovery::redeem_token::*;
use price_discovery::*;

use pair_mock::*;

const PD_WASM_PATH: &'static str = "../output/price-discovery.wasm";
const DEX_MOCK_WASM_PATH: &'static str = "../../pait-mock/output/pair_mock.wasm";

pub const LAUNCHED_TOKEN_ID: &[u8] = b"SOCOOLWOW-123456";
pub const ACCEPTED_TOKEN_ID: &[u8] = b"USDC-123456";
pub const REDEEM_TOKEN_ID: &[u8] = b"GIBREWARDS-123456";
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

pub const START_EPOCH: u64 = 5;
pub const END_EPOCH: u64 = 10;

pub struct PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub first_user_address: Address,
    pub second_user_address: Address,
    pub sc_dex_address: Address,
    pub pd_wrapper: ContractObjWrapper<price_discovery::ContractObj<DebugApi>, PriceDiscObjBuilder>,
    pub dex_wrapper: ContractObjWrapper<pair_mock::ContractObj<DebugApi>, DexObjBuilder>,
}

pub fn init<PriceDiscObjBuilder, DexObjBuilder>(
    pd_builder: PriceDiscObjBuilder,
    dex_builder: DexObjBuilder,
) -> PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_address = blockchain_wrapper.create_user_account(&rust_zero);
    let first_user_address = blockchain_wrapper.create_user_account(&rust_zero);
    let second_user_address = blockchain_wrapper.create_user_account(&rust_zero);

    let dex_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        dex_builder,
        DEX_MOCK_WASM_PATH,
    );

    // init DEX mock
    blockchain_wrapper
        .execute_tx(&owner_address, &dex_wrapper, &rust_zero, |sc| {
            sc.init(
                OptionalValue::Some(managed_token_id!(LAUNCHED_TOKEN_ID)),
                OptionalValue::Some(managed_token_id!(ACCEPTED_TOKEN_ID)),
                OptionalValue::None,
                OptionalValue::None,
                OptionalValue::None,
                OptionalValue::None,
                OptionalValue::None,
            );
        })
        .assert_ok();

    blockchain_wrapper.set_esdt_balance(
        &dex_wrapper.address_ref(),
        LP_TOKEN_ID,
        &rust_biguint!(500_000_000_000),
    );

    let pd_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        pd_builder,
        PD_WASM_PATH,
    );

    // set user balances
    blockchain_wrapper.set_esdt_balance(
        &owner_address,
        LAUNCHED_TOKEN_ID,
        &rust_biguint!(5_000_000_000),
    );
    blockchain_wrapper.set_esdt_balance(
        &first_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(1_000_000_000),
    );
    blockchain_wrapper.set_esdt_balance(
        &second_user_address,
        ACCEPTED_TOKEN_ID,
        &rust_biguint!(1_000_000_000),
    );

    // set sc roles and initial minted SFTs (only needed for the purpose of SFT add quantity)
    blockchain_wrapper.set_esdt_local_roles(
        pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::NftAddQuantity,
        ],
    );
    blockchain_wrapper.set_nft_balance(
        pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        LAUNCHED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(1),
        &(),
    );
    blockchain_wrapper.set_nft_balance(
        pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(1),
        &(),
    );

    blockchain_wrapper.set_block_epoch(START_EPOCH - 1);

    // init Price Discovery SC
    blockchain_wrapper
        .execute_tx(&owner_address, &pd_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(LAUNCHED_TOKEN_ID),
                managed_token_id!(ACCEPTED_TOKEN_ID),
                START_EPOCH,
                END_EPOCH,
            );

            sc.redeem_token_id()
                .set(&managed_token_id!(REDEEM_TOKEN_ID));
        })
        .assert_ok();

    let sc_dex_address = dex_wrapper.address_ref().clone();

    blockchain_wrapper
        .execute_tx(&owner_address, &pd_wrapper, &rust_zero, |sc| {
            sc.set_pair_address(ManagedAddress::from_address(&sc_dex_address));
        })
        .assert_ok();

    PriceDiscSetup {
        blockchain_wrapper,
        owner_address,
        first_user_address,
        second_user_address,
        sc_dex_address,
        pd_wrapper,
        dex_wrapper,
    }
}

pub fn call_deposit_initial_tokens<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    amount: &num_bigint::BigUint,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper
        .execute_esdt_transfer(
            &pd_setup.owner_address,
            &pd_setup.pd_wrapper,
            LAUNCHED_TOKEN_ID,
            0,
            amount,
            |sc| {
                sc.deposit();
            },
        )
        .assert_ok();
}

pub fn call_deposit<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    caller: &Address,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper.execute_esdt_transfer(
        caller,
        &pd_setup.pd_wrapper,
        ACCEPTED_TOKEN_ID,
        0,
        amount,
        |sc| {
            sc.deposit();
        },
    )
}

pub fn call_withdraw<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    caller: &Address,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper.execute_esdt_transfer(
        caller,
        &pd_setup.pd_wrapper,
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        amount,
        |sc| {
            let _ = sc.withdraw();
        },
    )
}

pub fn call_redeem<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    caller: &Address,
    sft_nonce: u64,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper.execute_esdt_transfer(
        caller,
        &pd_setup.pd_wrapper,
        REDEEM_TOKEN_ID,
        sft_nonce,
        amount,
        |sc| {
            sc.redeem();
        },
    )
}

pub fn call_create_dex_liquidity_pool<PriceDiscObjBuilder, DexObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder, DexObjBuilder>,
    caller: &Address,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    DexObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let b_wrapper = &mut pd_setup.blockchain_wrapper;
    b_wrapper.execute_tx(caller, &pd_setup.pd_wrapper, &rust_biguint!(0), |sc| {
        sc.create_dex_liquidity_pool();
    })
}
