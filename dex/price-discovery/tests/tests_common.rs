#![allow(deprecated)]

use multiversx_sc::codec::Empty;
use multiversx_sc::types::{Address, EsdtLocalRole};
use multiversx_sc_scenario::whitebox_legacy::TxResult;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, whitebox_legacy::*,
};
use multiversx_sc_scenario::{managed_token_id, rust_biguint, DebugApi};

use price_discovery::redeem_token::*;
use price_discovery::*;

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use simple_lock::locked_token::LockedTokenModule;
use simple_lock::SimpleLock;

const PD_WASM_PATH: &str = "../output/price-discovery.wasm";

pub const LAUNCHED_TOKEN_ID: &[u8] = b"SOCOOLWOW-123456";
pub const ACCEPTED_TOKEN_ID: &[u8] = b"USDC-123456";
pub const REDEEM_TOKEN_ID: &[u8] = b"GIBREWARDS-123456";
pub const LOCKED_TOKEN_ID: &[u8] = b"NOOO0-123456";
pub const OWNER_EGLD_BALANCE: u64 = 100_000_000;

pub const START_BLOCK: u64 = 10;
pub const NO_LIMIT_PHASE_DURATION_BLOCKS: u64 = 5;
pub const LINEAR_PENALTY_PHASE_DURATION_BLOCKS: u64 = 5;
pub const FIXED_PENALTY_PHASE_DURATION_BLOCKS: u64 = 5;
pub const END_BLOCK: u64 = START_BLOCK
    + NO_LIMIT_PHASE_DURATION_BLOCKS
    + LINEAR_PENALTY_PHASE_DURATION_BLOCKS
    + FIXED_PENALTY_PHASE_DURATION_BLOCKS;
pub const UNLOCK_EPOCH: u64 = 20;

pub const MIN_PENALTY_PERCENTAGE: u64 = 1_000_000_000_000; // 10%
pub const MAX_PENALTY_PERCENTAGE: u64 = 5_000_000_000_000; // 50%
pub const FIXED_PENALTY_PERCENTAGE: u64 = 2_500_000_000_000; // 25%

pub struct PriceDiscSetup<PriceDiscObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub first_user_address: Address,
    pub second_user_address: Address,
    pub pd_wrapper: ContractObjWrapper<price_discovery::ContractObj<DebugApi>, PriceDiscObjBuilder>,
    pub locking_sc_address: Address,
}

pub fn init<PriceDiscObjBuilder>(
    pd_builder: PriceDiscObjBuilder,
) -> PriceDiscSetup<PriceDiscObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let first_user_address = blockchain_wrapper.create_user_account(&rust_zero);
    let second_user_address = blockchain_wrapper.create_user_account(&rust_zero);
    let owner_address = blockchain_wrapper.create_user_account(&rust_biguint!(OWNER_EGLD_BALANCE));

    let pd_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        pd_builder,
        PD_WASM_PATH,
    );

    // set user balances
    let prev_owner_balance =
        blockchain_wrapper.get_esdt_balance(&owner_address, LAUNCHED_TOKEN_ID, 0);
    blockchain_wrapper.set_esdt_balance(
        &owner_address,
        LAUNCHED_TOKEN_ID,
        &(prev_owner_balance + rust_biguint!(5_000_000_000)),
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
        &Empty,
    );
    blockchain_wrapper.set_nft_balance(
        pd_wrapper.address_ref(),
        REDEEM_TOKEN_ID,
        ACCEPTED_TOKEN_REDEEM_NONCE,
        &rust_biguint!(1),
        &Empty,
    );

    blockchain_wrapper.set_block_nonce(START_BLOCK - 1);

    // init locking SC
    let locking_sc_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        simple_lock::contract_obj,
        "Some path",
    );

    blockchain_wrapper
        .execute_tx(&owner_address, &locking_sc_wrapper, &rust_zero, |sc| {
            sc.init();
            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
        })
        .assert_ok();

    blockchain_wrapper.set_esdt_local_roles(
        locking_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // init Price Discovery SC
    blockchain_wrapper
        .execute_tx(&owner_address, &pd_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(LAUNCHED_TOKEN_ID),
                managed_token_id_wrapped!(ACCEPTED_TOKEN_ID),
                18,
                managed_biguint!(0),
                START_BLOCK,
                NO_LIMIT_PHASE_DURATION_BLOCKS,
                LINEAR_PENALTY_PHASE_DURATION_BLOCKS,
                FIXED_PENALTY_PHASE_DURATION_BLOCKS,
                UNLOCK_EPOCH,
                managed_biguint!(MIN_PENALTY_PERCENTAGE),
                managed_biguint!(MAX_PENALTY_PERCENTAGE),
                managed_biguint!(FIXED_PENALTY_PERCENTAGE),
                managed_address!(locking_sc_wrapper.address_ref()),
            );

            sc.redeem_token()
                .set_token_id(managed_token_id!(REDEEM_TOKEN_ID));
        })
        .assert_ok();

    PriceDiscSetup {
        blockchain_wrapper,
        owner_address,
        first_user_address,
        second_user_address,
        pd_wrapper,
        locking_sc_address: locking_sc_wrapper.address_ref().clone(),
    }
}

pub fn call_deposit_initial_tokens<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
    amount: &num_bigint::BigUint,
) where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
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

pub fn call_deposit<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
    caller: &Address,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
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

pub fn call_withdraw<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
    caller: &Address,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
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

pub fn call_redeem<PriceDiscObjBuilder>(
    pd_setup: &mut PriceDiscSetup<PriceDiscObjBuilder>,
    caller: &Address,
    sft_nonce: u64,
    amount: &num_bigint::BigUint,
) -> TxResult
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
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
