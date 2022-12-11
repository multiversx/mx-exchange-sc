use elrond_wasm::elrond_codec::multi_types::MultiValue3;
use elrond_wasm::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use elrond_wasm_debug::tx_mock::TxInputESDT;
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};

pub const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

pub const LOCKED_TOKEN_ID: &[u8] = b"LOCKED-abcdef";
pub const LP_PROXY_TOKEN_ID: &[u8] = b"LPPROXY-abcdef";

pub const USER_TOTAL_MEX_TOKENS: u64 = 5_000_000_000;
pub const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;

use pair::bot_protection::*;
use pair::config::ConfigModule as PairConfigModule;
use pair::safe_price::*;
use pair::*;
use pausable::{PausableModule, State};

#[allow(dead_code)]
pub struct PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

impl<PairObjBuilder> PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub fn new(pair_builder: PairObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let pair_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_addr), pair_builder, PAIR_WASM_PATH);

        b_mock
            .execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
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

                let lp_token_id = managed_token_id!(LP_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(&State::Active);
                sc.set_max_observations_per_record(10);
            })
            .assert_ok();

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.set_esdt_local_roles(pair_wrapper.address_ref(), LP_TOKEN_ID, &lp_token_roles[..]);

        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));
        b_mock.set_esdt_balance(
            &user_addr,
            WEGLD_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
        );
        b_mock.set_esdt_balance(
            &user_addr,
            MEX_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_MEX_TOKENS),
        );

        PairSetup {
            b_mock,
            owner_address: owner_addr,
            user_address: user_addr,
            pair_wrapper,
        }
    }

    pub fn add_liquidity(
        &mut self,
        first_token_amount: u64,
        first_token_min: u64,
        second_token_amount: u64,
        second_token_min: u64,
        expected_lp_amount: u64,
        expected_first_amount: u64,
        expected_second_amount: u64,
    ) {
        let payments = vec![
            TxInputESDT {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxInputESDT {
                token_identifier: MEX_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(second_token_amount),
            },
        ];

        self.b_mock
            .execute_esdt_multi_transfer(&self.user_address, &self.pair_wrapper, &payments, |sc| {
                let MultiValue3 { 0: payments } = sc.add_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                );

                assert_eq!(payments.0.token_identifier, managed_token_id!(LP_TOKEN_ID));
                assert_eq!(payments.0.token_nonce, 0);
                assert_eq!(payments.0.amount, managed_biguint!(expected_lp_amount));

                assert_eq!(
                    payments.1.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(payments.1.token_nonce, 0);
                assert_eq!(payments.1.amount, managed_biguint!(expected_first_amount));

                assert_eq!(payments.2.token_identifier, managed_token_id!(MEX_TOKEN_ID));
                assert_eq!(payments.2.token_nonce, 0);
                assert_eq!(payments.2.amount, managed_biguint!(expected_second_amount));
            })
            .assert_ok();
    }

    pub fn swap_fixed_input(
        &mut self,
        payment_token_id: &[u8],
        payment_amount: u64,
        desired_token_id: &[u8],
        desired_amount_min: u64,
        expected_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.pair_wrapper,
                &payment_token_id,
                0,
                &rust_biguint!(payment_amount),
                |sc| {
                    let ret = sc.swap_tokens_fixed_input(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount_min),
                    );

                    assert_eq!(ret.token_identifier, managed_token_id!(desired_token_id));
                    assert_eq!(ret.token_nonce, 0);
                    assert_eq!(ret.amount, managed_biguint!(expected_amount));
                },
            )
            .assert_ok();
    }

    pub fn swap_fixed_input_expect_error(
        &mut self,
        payment_token_id: &[u8],
        payment_amount: u64,
        desired_token_id: &[u8],
        desired_amount_min: u64,
        expected_message: &str,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.pair_wrapper,
                &payment_token_id,
                0,
                &rust_biguint!(payment_amount),
                |sc| {
                    sc.swap_tokens_fixed_input(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount_min),
                    );
                },
            )
            .assert_user_error(expected_message);
    }

    pub fn swap_fixed_output(
        &mut self,
        payment_token_id: &[u8],
        payment_amount_max: u64,
        desired_token_id: &[u8],
        desired_amount: u64,
        payment_expected_back_amount: u64,
    ) {
        let initial_payment_token_balance =
            self.b_mock
                .get_esdt_balance(&self.user_address, payment_token_id, 0);
        let initial_desired_token_balance =
            self.b_mock
                .get_esdt_balance(&self.user_address, desired_token_id, 0);

        let mut payment_token_swap_amount = rust_biguint!(0);
        let mut desired_token_swap_amount = rust_biguint!(0);

        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.pair_wrapper,
                &payment_token_id,
                0,
                &rust_biguint!(payment_amount_max),
                |sc| {
                    let ret = sc.swap_tokens_fixed_output(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount),
                    );

                    let (desired_token_output, payment_token_residuum) = ret.into_tuple();
                    payment_token_swap_amount = num_bigint::BigUint::from_bytes_be(
                        &payment_token_residuum.amount.to_bytes_be().as_slice(),
                    );
                    desired_token_swap_amount = num_bigint::BigUint::from_bytes_be(
                        &desired_token_output.amount.to_bytes_be().as_slice(),
                    );

                    assert_eq!(
                        payment_token_residuum.amount,
                        managed_biguint!(payment_expected_back_amount)
                    );
                },
            )
            .assert_ok();

        let final_payment_token_balance =
            self.b_mock
                .get_esdt_balance(&self.user_address, payment_token_id, 0);
        let final_desired_token_balance =
            self.b_mock
                .get_esdt_balance(&self.user_address, desired_token_id, 0);

        assert_eq!(
            final_payment_token_balance,
            initial_payment_token_balance - &rust_biguint!(payment_amount_max)
                + payment_token_swap_amount
        );

        assert_eq!(
            final_desired_token_balance,
            initial_desired_token_balance + desired_token_swap_amount
        );
    }

    pub fn set_swap_protect(
        &mut self,
        protect_stop_block: u64,
        volume_percent: u64,
        max_num_actions_per_address: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner_address,
                &self.pair_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_bp_swap_config(
                        protect_stop_block,
                        volume_percent,
                        max_num_actions_per_address,
                    );
                },
            )
            .assert_ok();
    }

    pub fn check_current_safe_state(
        &mut self,
        from: u64,
        to: u64,
        num_obs: u64,
        first_reserve_last_obs: u64,
        second_reserve_last_obs: u64,
        first_reserve_weighted: u64,
        second_reserve_weighted: u64,
    ) {
        self.b_mock
            .execute_query(&self.pair_wrapper, |sc| {
                let state = sc.get_current_state_or_default();

                assert_eq!(state.first_obs_block, from);
                assert_eq!(state.last_obs_block, to);
                assert_eq!(state.num_observations, num_obs);
                assert_eq!(
                    state.first_token_reserve_last_obs,
                    managed_biguint!(first_reserve_last_obs)
                );
                assert_eq!(
                    state.second_token_reserve_last_obs,
                    managed_biguint!(second_reserve_last_obs)
                );
                assert_eq!(
                    state.first_token_reserve_weighted,
                    managed_biguint!(first_reserve_weighted)
                );
                assert_eq!(
                    state.second_token_reserve_weighted,
                    managed_biguint!(second_reserve_weighted)
                );
            })
            .assert_ok();
    }

    pub fn check_future_safe_state(
        &mut self,
        from: u64,
        to: u64,
        num_obs: u64,
        first_reserve_last_obs: u64,
        second_reserve_last_obs: u64,
        first_reserve_weighted: u64,
        second_reserve_weighted: u64,
    ) {
        self.b_mock
            .execute_query(&self.pair_wrapper, |sc| {
                let state = sc.get_future_state_or_default();

                assert_eq!(state.first_obs_block, from);
                assert_eq!(state.last_obs_block, to);
                assert_eq!(state.num_observations, num_obs);
                assert_eq!(
                    state.first_token_reserve_last_obs,
                    managed_biguint!(first_reserve_last_obs)
                );
                assert_eq!(
                    state.second_token_reserve_last_obs,
                    managed_biguint!(second_reserve_last_obs)
                );
                assert_eq!(
                    state.first_token_reserve_weighted,
                    managed_biguint!(first_reserve_weighted)
                );
                assert_eq!(
                    state.second_token_reserve_weighted,
                    managed_biguint!(second_reserve_weighted)
                );
            })
            .assert_ok();
    }
}
