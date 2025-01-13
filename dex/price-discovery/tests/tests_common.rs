#![allow(deprecated)]

use multiversx_sc::types::{Address, EsdtLocalRole};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, whitebox_legacy::*,
};
use multiversx_sc_scenario::{managed_token_id, rust_biguint, DebugApi};

use price_discovery::redeem_token::*;
use price_discovery::*;

use multiversx_sc::storage::mappers::StorageTokenWrapper;
use user_actions::owner_deposit_withdraw::OwnerDepositWithdrawModule;
use user_actions::redeem::RedeemModule;
use user_actions::user_deposit_withdraw::UserDepositWithdrawModule;

static PD_WASM_PATH: &str = "../output/price-discovery.wasm";

pub static LAUNCHED_TOKEN_ID: &[u8] = b"SOCOOLWOW-123456";
pub static ACCEPTED_TOKEN_ID: &[u8] = b"USDC-123456";
pub static REDEEM_TOKEN_ID: &[u8] = b"GIBREWARDS-123456";
pub const OWNER_EGLD_BALANCE: u64 = 100_000_000;
pub const USER_BALANCE: u64 = 1_000_000_000;

pub const START_TIME: Timestamp = 10;
pub const USER_DEPOSIT_TIME: Timestamp = 5;
pub const OWNER_DEPOSIT_TIME: Timestamp = 5;
// pub const END_TIME: u64 = START_TIME + USER_DEPOSIT_TIME + OWNER_DEPOSIT_TIME;
pub const MIN_LAUNCHED_TOKENS: u64 = 1_000;

pub struct PriceDiscSetup<PriceDiscObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub first_user_address: Address,
    pub second_user_address: Address,
    pub pd_wrapper: ContractObjWrapper<price_discovery::ContractObj<DebugApi>, PriceDiscObjBuilder>,
}

impl<PriceDiscObjBuilder> PriceDiscSetup<PriceDiscObjBuilder>
where
    PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
{
    pub fn new(pd_builder: PriceDiscObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let first_user_address = b_mock.create_user_account(&rust_zero);
        let second_user_address = b_mock.create_user_account(&rust_zero);
        let owner_address = b_mock.create_user_account(&rust_biguint!(OWNER_EGLD_BALANCE));

        let pd_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_address), pd_builder, PD_WASM_PATH);

        // set user balances
        let prev_owner_balance = b_mock.get_esdt_balance(&owner_address, LAUNCHED_TOKEN_ID, 0);
        b_mock.set_esdt_balance(
            &owner_address,
            LAUNCHED_TOKEN_ID,
            &(prev_owner_balance + rust_biguint!(USER_BALANCE)),
        );
        b_mock.set_esdt_balance(
            &first_user_address,
            ACCEPTED_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
        );
        b_mock.set_esdt_balance(
            &second_user_address,
            ACCEPTED_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
        );

        // set sc roles and initial minted SFTs (only needed for the purpose of SFT add quantity)
        b_mock.set_esdt_local_roles(
            pd_wrapper.address_ref(),
            REDEEM_TOKEN_ID,
            &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
        );

        b_mock.set_block_timestamp(START_TIME - 1);

        // init Price Discovery SC
        b_mock
            .execute_tx(&owner_address, &pd_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(LAUNCHED_TOKEN_ID),
                    managed_token_id_wrapped!(ACCEPTED_TOKEN_ID),
                    18,
                    managed_biguint!(MIN_LAUNCHED_TOKENS),
                    START_TIME,
                    USER_DEPOSIT_TIME,
                    OWNER_DEPOSIT_TIME,
                    managed_address!(&owner_address),
                );

                sc.redeem_token()
                    .set_token_id(managed_token_id!(REDEEM_TOKEN_ID));
            })
            .assert_ok();

        PriceDiscSetup {
            b_mock,
            owner_address,
            first_user_address,
            second_user_address,
            pd_wrapper,
        }
    }

    pub fn call_user_deposit(&mut self, user: &Address, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            user,
            &self.pd_wrapper,
            ACCEPTED_TOKEN_ID,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.user_deposit();
            },
        )
    }

    pub fn call_user_withdraw(&mut self, user: &Address, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            user,
            &self.pd_wrapper,
            REDEEM_TOKEN_ID,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.user_withdraw();
            },
        )
    }

    pub fn call_owner_deposit(&mut self, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &self.owner_address,
            &self.pd_wrapper,
            LAUNCHED_TOKEN_ID,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.owner_deposit();
            },
        )
    }

    pub fn call_owner_withdraw(&mut self, amount: u64) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.pd_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.owner_withdraw(managed_biguint!(amount));
            },
        )
    }

    pub fn call_user_redeem(&mut self, user: &Address, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            user,
            &self.pd_wrapper,
            REDEEM_TOKEN_ID,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.redeem();
            },
        )
    }

    pub fn call_owner_redeem(&mut self) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.pd_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.redeem();
            },
        )
    }
}
