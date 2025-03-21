use std::cell::RefCell;
use std::rc::Rc;

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub const PAIR_WASM_PATH: &str = "pair/output/pair.wasm";
pub const ROUTER_WASM_PATH: &str = "router/output/router.wasm";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const USDC_TOKEN_ID: &[u8] = b"USDC-abcdef";
pub const LPMEX_TOKEN_ID: &[u8] = b"LPMEX-abcdef";
pub const LPUSDC_TOKEN_ID: &[u8] = b"LPUSDC-abcdef";

pub const USER_TOTAL_MEX_TOKENS: u64 = 5_001_001_000;
pub const USER_TOTAL_WEGLD_TOKENS: u64 = 5_002_002_000;
pub const USER_TOTAL_USDC_TOKENS: u64 = 5_001_001_000;

pub const ADD_LIQUIDITY_TOKENS: u64 = 1_001_000;

use pair::config::ConfigModule as PairConfigModule;
use pair::pair_actions::add_liq::AddLiquidityModule;
use pair::*;
use pausable::{PausableModule, State};
use router::config::ConfigModule;
use router::factory::*;
use router::*;

#[allow(dead_code)]
pub struct RouterSetup<RouterObjBuilder, PairObjBuilder>
where
    RouterObjBuilder: 'static + Copy + Fn() -> router::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub owner_address: Address,
    pub user_address: Address,
    pub router_wrapper: ContractObjWrapper<router::ContractObj<DebugApi>, RouterObjBuilder>,
    pub wegld_mex_pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub wegld_usdc_pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

impl<RouterObjBuilder, PairObjBuilder> RouterSetup<RouterObjBuilder, PairObjBuilder>
where
    RouterObjBuilder: 'static + Copy + Fn() -> router::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        router_builder: RouterObjBuilder,
        pair_builder: PairObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let owner_addr = b_mock.borrow_mut().create_user_account(&rust_zero);

        let router_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            router_builder,
            ROUTER_WASM_PATH,
        );

        let wegld_mex_pair_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            pair_builder,
            PAIR_WASM_PATH,
        );

        let wegld_usdc_pair_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            pair_builder,
            PAIR_WASM_PATH,
        );

        b_mock
            .borrow_mut()
            .execute_tx(&owner_addr, &wegld_mex_pair_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(MEX_TOKEN_ID);
                let router_address = managed_address!(&owner_addr);
                let router_owner_address = managed_address!(&owner_addr);
                let total_fee_percent = 300u64;
                let special_fee_percent = 50u64;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                let lp_token_id = managed_token_id!(LPMEX_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(State::Active);
            })
            .assert_ok();

        b_mock
            .borrow_mut()
            .execute_tx(&owner_addr, &wegld_usdc_pair_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(USDC_TOKEN_ID);
                let router_address = managed_address!(&owner_addr);
                let router_owner_address = managed_address!(&owner_addr);
                let total_fee_percent = 300u64;
                let special_fee_percent = 50u64;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                let lp_token_id = managed_token_id!(LPUSDC_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(State::Active);
            })
            .assert_ok();

        b_mock
            .borrow_mut()
            .execute_tx(&owner_addr, &router_wrapper, &rust_zero, |sc| {
                sc.init(OptionalValue::None);

                sc.pair_map().insert(
                    PairTokens {
                        first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
                        second_token_id: managed_token_id!(MEX_TOKEN_ID),
                    },
                    managed_address!(wegld_mex_pair_wrapper.address_ref()),
                );
                sc.pair_map().insert(
                    PairTokens {
                        first_token_id: managed_token_id!(WEGLD_TOKEN_ID),
                        second_token_id: managed_token_id!(USDC_TOKEN_ID),
                    },
                    managed_address!(wegld_usdc_pair_wrapper.address_ref()),
                );
            })
            .assert_ok();

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.borrow_mut().set_esdt_local_roles(
            wegld_mex_pair_wrapper.address_ref(),
            LPMEX_TOKEN_ID,
            &lp_token_roles[..],
        );

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.borrow_mut().set_esdt_local_roles(
            wegld_usdc_pair_wrapper.address_ref(),
            LPUSDC_TOKEN_ID,
            &lp_token_roles[..],
        );

        let user_addr = b_mock
            .borrow_mut()
            .create_user_account(&rust_biguint!(100_000_000));
        b_mock.borrow_mut().set_esdt_balance(
            &user_addr,
            WEGLD_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
        );
        b_mock.borrow_mut().set_esdt_balance(
            &user_addr,
            MEX_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_MEX_TOKENS),
        );
        b_mock.borrow_mut().set_esdt_balance(
            &user_addr,
            USDC_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_USDC_TOKENS),
        );

        RouterSetup {
            b_mock,
            owner_address: owner_addr,
            user_address: user_addr,
            router_wrapper,
            wegld_mex_pair_wrapper,
            wegld_usdc_pair_wrapper,
        }
    }

    // TODO: Maybe change token amounts
    pub fn add_liquidity(&mut self) {
        let payments = vec![
            TxTokenTransfer {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(ADD_LIQUIDITY_TOKENS),
            },
            TxTokenTransfer {
                token_identifier: MEX_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(ADD_LIQUIDITY_TOKENS),
            },
        ];

        self.b_mock
            .borrow_mut()
            .execute_esdt_multi_transfer(
                &self.user_address,
                &self.wegld_mex_pair_wrapper,
                &payments,
                |sc| {
                    sc.add_liquidity(
                        managed_biguint!(ADD_LIQUIDITY_TOKENS),
                        managed_biguint!(ADD_LIQUIDITY_TOKENS),
                    );
                },
            )
            .assert_ok();

        let payments = vec![
            TxTokenTransfer {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(ADD_LIQUIDITY_TOKENS),
            },
            TxTokenTransfer {
                token_identifier: USDC_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(ADD_LIQUIDITY_TOKENS),
            },
        ];

        self.b_mock
            .borrow_mut()
            .execute_esdt_multi_transfer(
                &self.user_address,
                &self.wegld_usdc_pair_wrapper,
                &payments,
                |sc| {
                    sc.add_liquidity(
                        managed_biguint!(ADD_LIQUIDITY_TOKENS),
                        managed_biguint!(ADD_LIQUIDITY_TOKENS),
                    );
                },
            )
            .assert_ok();
    }
}
