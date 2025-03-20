#![allow(deprecated)]

use multiversx_sc::types::{Address, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, whitebox_legacy::*,
};
use multiversx_sc_scenario::{managed_token_id, rust_biguint, DebugApi};

use price_discovery::common_storage::CommonStorageModule;
use price_discovery::user_actions::admin_actions::AdminActionsModule;
use price_discovery::*;

use user_actions::owner_deposit_withdraw::OwnerDepositWithdrawModule;
use user_actions::redeem::RedeemModule;
use user_actions::user_deposit_withdraw::UserDepositWithdrawModule;

static PD_WASM_PATH: &str = "../output/price-discovery.wasm";

pub static LAUNCHED_TOKEN_ID: &[u8] = b"SOCOOLWOW-123456";
pub static ACCEPTED_TOKEN_ID: &[u8] = b"USDC-123456";
pub const OWNER_EGLD_BALANCE: u64 = 100_000_000;
pub const USER_BALANCE: u64 = 1_000_000_000;

pub const START_TIME: Timestamp = 10;
pub const USER_DEPOSIT_TIME: Timestamp = 100;
pub const OWNER_DEPOSIT_TIME: Timestamp = 100;
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

        b_mock.set_block_timestamp(START_TIME - 1);

        // init Price Discovery SC
        b_mock
            .execute_tx(&owner_address, &pd_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(LAUNCHED_TOKEN_ID),
                    managed_token_id_wrapped!(ACCEPTED_TOKEN_ID),
                    18,
                    START_TIME,
                    USER_DEPOSIT_TIME,
                    OWNER_DEPOSIT_TIME,
                    managed_biguint!(100),
                    managed_address!(&owner_address),
                );

                sc.min_launched_tokens()
                    .set(managed_biguint!(MIN_LAUNCHED_TOKENS));

                let mut pairs = MultiValueEncoded::new();
                pairs.push((managed_address!(&first_user_address), managed_biguint!(0)).into());
                pairs.push(
                    (
                        managed_address!(&second_user_address),
                        managed_biguint!(10_000),
                    )
                        .into(),
                );
                sc.add_users_to_whitelist(pairs);
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
        self.b_mock
            .execute_tx(user, &self.pd_wrapper, &rust_biguint!(0), |sc| {
                sc.user_withdraw_endpoint(managed_biguint!(amount));
            })
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

    pub fn call_user_redeem(&mut self, user: &Address) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.pd_wrapper, &rust_biguint!(0), |sc| {
                sc.redeem();
            })
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

    pub fn call_refund_user(&mut self, user: &Address) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.pd_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut users = MultiValueEncoded::new();
                users.push(managed_address!(user));

                sc.refund_users(users);
            },
        )
    }

    pub fn call_set_user_limit(&mut self, user: &Address, limit: u64) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.pd_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_user_limit(managed_address!(user), managed_biguint!(limit));
            },
        )
    }

    pub fn call_set_user_deposit_withdraw_timestamp(&mut self, timestamp: Timestamp) -> TxResult {
        self.b_mock.execute_tx(
            &self.owner_address,
            &self.pd_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.set_user_deposit_withdraw_time(timestamp);
            },
        )
    }
}
