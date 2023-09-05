use multiversx_sc::codec::multi_types::MultiValue3;
use multiversx_sc::types::{
    Address, EsdtLocalRole, EsdtTokenPayment, ManagedAddress, MultiValueEncoded,
};
use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub const PAIR_WASM_PATH: &str = "pair/output/pair.wasm";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
pub const OTHER_TOKEN_ID: &[u8] = b"OTHER-abcdef";
pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

pub const LOCKED_TOKEN_ID: &[u8] = b"LOCKED-abcdef";
pub const LP_PROXY_TOKEN_ID: &[u8] = b"LPPROXY-abcdef";

pub const USER_TOTAL_MEX_TOKENS: u64 = 5_000_000_000;
pub const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;

use pair::config::ConfigModule as PairConfigModule;
use pair::safe_price_view::*;
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
    pub second_pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
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

        let second_pair_wrapper =
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

                sc.state().set(State::Active);
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner_addr, &second_pair_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(OTHER_TOKEN_ID);
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

                sc.state().set(State::Active);
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
            second_pair_wrapper,
        }
    }

    #[allow(clippy::too_many_arguments)]
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
            TxTokenTransfer {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxTokenTransfer {
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
                payment_token_id,
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
                payment_token_id,
                0,
                &rust_biguint!(payment_amount_max),
                |sc| {
                    let ret = sc.swap_tokens_fixed_output(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount),
                    );

                    let (desired_token_output, payment_token_residuum) = ret.into_tuple();
                    payment_token_swap_amount = num_bigint::BigUint::from_bytes_be(
                        payment_token_residuum.amount.to_bytes_be().as_slice(),
                    );
                    desired_token_swap_amount = num_bigint::BigUint::from_bytes_be(
                        desired_token_output.amount.to_bytes_be().as_slice(),
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

    pub fn check_price_observation(
        &mut self,
        pair_address: &Address,
        search_round: u64,
        weight_accumulated: u64,
        first_token_reserve_accumulated: u64,
        second_token_reserve_accumulated: u64,
    ) {
        self.b_mock
            .execute_query(&self.pair_wrapper, |sc| {
                let price_observation =
                    sc.get_price_observation_view(managed_address!(pair_address), search_round);
                assert_eq!(price_observation.weight_accumulated, weight_accumulated);
                assert_eq!(
                    price_observation.first_token_reserve_accumulated,
                    managed_biguint!(first_token_reserve_accumulated)
                );
                assert_eq!(
                    price_observation.second_token_reserve_accumulated,
                    managed_biguint!(second_token_reserve_accumulated)
                );
            })
            .assert_ok();
    }

    pub fn check_price_observation_from_second_pair(
        &mut self,
        pair_address: &Address,
        search_round: u64,
        weight_accumulated: u64,
        first_token_reserve_accumulated: u64,
        second_token_reserve_accumulated: u64,
    ) {
        self.b_mock
            .execute_query(&self.second_pair_wrapper, |sc| {
                let price_observation =
                    sc.get_price_observation_view(managed_address!(pair_address), search_round);
                assert_eq!(price_observation.weight_accumulated, weight_accumulated);
                assert_eq!(
                    price_observation.first_token_reserve_accumulated,
                    managed_biguint!(first_token_reserve_accumulated)
                );
                assert_eq!(
                    price_observation.second_token_reserve_accumulated,
                    managed_biguint!(second_token_reserve_accumulated)
                );
            })
            .assert_ok();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_safe_price(
        &mut self,
        pair_address: &Address,
        start_round: u64,
        end_round: u64,
        payment_token_id: &[u8],
        payment_token_amount: u64,
        expected_token_id: &[u8],
        expected_token_amount: u64,
    ) {
        let _ = self.b_mock.execute_query(&self.pair_wrapper, |sc| {
            let input_payment = EsdtTokenPayment::new(
                managed_token_id!(payment_token_id),
                0,
                managed_biguint!(payment_token_amount),
            );
            let expected_payment = sc.get_safe_price(
                managed_address!(pair_address),
                start_round,
                end_round,
                input_payment,
            );
            assert_eq!(
                expected_payment.token_identifier,
                managed_token_id!(expected_token_id)
            );
            assert_eq!(
                expected_payment.amount,
                managed_biguint!(expected_token_amount)
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_safe_price_from_second_pair(
        &mut self,
        pair_address: &Address,
        start_round: u64,
        end_round: u64,
        payment_token_id: &[u8],
        payment_token_amount: u64,
        expected_token_id: &[u8],
        expected_token_amount: u64,
    ) {
        let _ = self.b_mock.execute_query(&self.second_pair_wrapper, |sc| {
            let input_payment = EsdtTokenPayment::new(
                managed_token_id!(payment_token_id),
                0,
                managed_biguint!(payment_token_amount),
            );
            let expected_payment = sc.get_safe_price(
                managed_address!(pair_address),
                start_round,
                end_round,
                input_payment,
            );
            assert_eq!(
                expected_payment.token_identifier,
                managed_token_id!(expected_token_id)
            );
            assert_eq!(
                expected_payment.amount,
                managed_biguint!(expected_token_amount)
            );
        });
    }

    pub fn check_safe_price_from_legacy_endpoint(
        &mut self,
        payment_token_id: &[u8],
        payment_token_amount: u64,
        expected_token_id: &[u8],
        expected_token_amount: u64,
    ) {
        let _ = self.b_mock.execute_query(&self.pair_wrapper, |sc| {
            let input_payment = EsdtTokenPayment::new(
                managed_token_id!(payment_token_id),
                0,
                managed_biguint!(payment_token_amount),
            );
            let expected_payment = sc.update_and_get_safe_price(input_payment);
            assert_eq!(
                expected_payment.token_identifier,
                managed_token_id!(expected_token_id)
            );
            assert_eq!(
                expected_payment.amount,
                managed_biguint!(expected_token_amount)
            );
        });
    }
}
