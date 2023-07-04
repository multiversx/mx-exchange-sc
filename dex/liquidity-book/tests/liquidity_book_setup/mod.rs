use liquidity_book::lp_token::LpTokenModule;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, BigUint, EsdtLocalRole};
use multiversx_sc_scenario::whitebox::TxTokenTransfer;
use multiversx_sc_scenario::{managed_token_id, num_bigint, rust_biguint, whitebox::*, DebugApi};

pub const LIQUIDITY_BOOK_WASM_PATH: &str = "liquidity-book/output/liquidity-book.wasm";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

pub const USER_TOTAL_MEX_TOKENS: u64 = 5_000_000_000;
pub const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;

pub const SWAP_FEE_PERCENTAGE: u64 = 30; // 0.3%
use liquidity_book::*;
use pausable::{PausableModule, State};

#[allow(dead_code)]
pub struct LiquidityBookSetup<LiquidityBookObjBuilder>
where
    LiquidityBookObjBuilder: 'static + Copy + Fn() -> liquidity_book::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub liquidity_book_wrapper:
        ContractObjWrapper<liquidity_book::ContractObj<DebugApi>, LiquidityBookObjBuilder>,
}

impl<LiquidityBookObjBuilder> LiquidityBookSetup<LiquidityBookObjBuilder>
where
    LiquidityBookObjBuilder: 'static + Copy + Fn() -> liquidity_book::ContractObj<DebugApi>,
{
    pub fn new(deploy_price: u128, liquidity_book_builder: LiquidityBookObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let liquidity_book_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            liquidity_book_builder,
            LIQUIDITY_BOOK_WASM_PATH,
        );

        b_mock
            .execute_tx(&owner_addr, &liquidity_book_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(MEX_TOKEN_ID);

                sc.init(
                    first_token_id,
                    second_token_id,
                    BigUint::from(deploy_price),
                    BigUint::zero(),
                    SWAP_FEE_PERCENTAGE,
                );

                sc.lp_token().set_token_id(managed_token_id!(LP_TOKEN_ID));

                sc.state().set(State::Active);
            })
            .assert_ok();

        let lp_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        b_mock.set_esdt_local_roles(
            liquidity_book_wrapper.address_ref(),
            LP_TOKEN_ID,
            &lp_token_roles[..],
        );

        LiquidityBookSetup {
            b_mock,
            owner_address: owner_addr,
            liquidity_book_wrapper,
        }
    }

    pub fn add_liquidity(
        &mut self,
        user_address: &Address,
        min_price: u128,
        max_price: u128,
        first_token_amount: u128,
        second_token_amount: u128,
        expected_lp_token_nonce: u64,
        expected_lp_amount: u128,
        expected_first_amount: u128,
        expected_second_amount: u128,
    ) {
        let payments = vec![
            TxTokenTransfer {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: num_bigint::BigUint::from(first_token_amount),
            },
            TxTokenTransfer {
                token_identifier: MEX_TOKEN_ID.to_vec(),
                nonce: 0,
                value: num_bigint::BigUint::from(second_token_amount),
            },
        ];

        self.b_mock
            .execute_esdt_multi_transfer(
                user_address,
                &self.liquidity_book_wrapper,
                &payments,
                |sc| {
                    let (
                        lp_token_payment_output,
                        first_token_payment_output,
                        second_token_payment_output,
                    ) = sc
                        .add_liquidity(BigUint::from(min_price), BigUint::from(max_price))
                        .into_tuple();

                    assert_eq!(
                        lp_token_payment_output.token_identifier,
                        managed_token_id!(LP_TOKEN_ID)
                    );
                    assert_eq!(lp_token_payment_output.token_nonce, expected_lp_token_nonce);
                    assert_eq!(
                        lp_token_payment_output.amount,
                        BigUint::from(expected_lp_amount)
                    );

                    assert_eq!(
                        first_token_payment_output.token_identifier,
                        managed_token_id!(WEGLD_TOKEN_ID)
                    );
                    assert_eq!(first_token_payment_output.token_nonce, 0);
                    assert_eq!(
                        first_token_payment_output.amount,
                        BigUint::from(expected_first_amount)
                    );

                    assert_eq!(
                        second_token_payment_output.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_token_payment_output.token_nonce, 0);
                    assert_eq!(
                        second_token_payment_output.amount,
                        BigUint::from(expected_second_amount)
                    );
                },
            )
            .assert_ok();
    }

    pub fn remove_liquidity(
        &mut self,
        user_address: &Address,
        lp_token_nonce: u64,
        lp_token_amount_sqrt: u128,
        expected_first_amount: u128,
        expected_second_amount: u128,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                user_address,
                &self.liquidity_book_wrapper,
                LP_TOKEN_ID,
                lp_token_nonce,
                &num_bigint::BigUint::from(lp_token_amount_sqrt),
                |sc| {
                    let (first_token_payment_output, second_token_payment_output) =
                        sc.remove_liquidity().into_tuple();
                    assert_eq!(
                        first_token_payment_output.token_identifier,
                        managed_token_id!(WEGLD_TOKEN_ID)
                    );
                    assert_eq!(first_token_payment_output.token_nonce, 0);
                    assert_eq!(
                        first_token_payment_output.amount,
                        BigUint::from(expected_first_amount)
                    );

                    assert_eq!(
                        second_token_payment_output.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_token_payment_output.token_nonce, 0);
                    assert_eq!(
                        second_token_payment_output.amount,
                        BigUint::from(expected_second_amount)
                    );
                },
            )
            .assert_ok();
    }

    pub fn swap_tokens(
        &mut self,
        user_address: &Address,
        payment_token_id: &[u8],
        payment_amount: u128,
        desired_token_id: &[u8],
        expected_amount: u128,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                user_address,
                &self.liquidity_book_wrapper,
                payment_token_id,
                0,
                &num_bigint::BigUint::from(payment_amount),
                |sc| {
                    let output = sc.swap_tokens();

                    assert_eq!(output.token_identifier, managed_token_id!(desired_token_id));
                    assert_eq!(output.token_nonce, 0);
                    assert_eq!(output.amount, BigUint::from(expected_amount));
                },
            )
            .assert_ok();
    }

    pub fn setup_user(&mut self, first_token_amount: u64, second_token_amount: u64) -> Address {
        let user_addr = self
            .b_mock
            .create_user_account(&num_bigint::BigUint::from(100_000_000u128));
        self.b_mock.set_esdt_balance(
            &user_addr,
            WEGLD_TOKEN_ID,
            &(rust_biguint!(first_token_amount) * PRICE_DECIMALS),
        );
        self.b_mock.set_esdt_balance(
            &user_addr,
            MEX_TOKEN_ID,
            &(rust_biguint!(second_token_amount) * PRICE_DECIMALS),
        );
        user_addr
    }

    // pub fn swap_fixed_output(
    //     &mut self,
    //     payment_token_id: &[u8],
    //     payment_amount_max: u64,
    //     desired_token_id: &[u8],
    //     desired_amount: u64,
    //     payment_expected_back_amount: u64,
    // ) {
    //     let initial_payment_token_balance =
    //         self.b_mock
    //             .get_esdt_balance(&self.user_address, payment_token_id, 0);
    //     let initial_desired_token_balance =
    //         self.b_mock
    //             .get_esdt_balance(&self.user_address, desired_token_id, 0);

    //     let mut payment_token_swap_amount = rust_biguint!(0);
    //     let mut desired_token_swap_amount = rust_biguint!(0);

    //     self.b_mock
    //         .execute_esdt_transfer(
    //             &self.user_address,
    //             &self.liquidity_book_wrapper,
    //             payment_token_id,
    //             0,
    //             &rust_biguint!(payment_amount_max),
    //             |sc| {
    //                 let ret = sc.swap_tokens_fixed_output(
    //                     managed_token_id!(desired_token_id),
    //                     managed_biguint!(desired_amount),
    //                 );

    //                 let (desired_token_output, payment_token_residuum) = ret.into_tuple();
    //                 payment_token_swap_amount = num_bigint::BigUint::from_bytes_be(
    //                     payment_token_residuum.amount.to_bytes_be().as_slice(),
    //                 );
    //                 desired_token_swap_amount = num_bigint::BigUint::from_bytes_be(
    //                     desired_token_output.amount.to_bytes_be().as_slice(),
    //                 );

    //                 assert_eq!(
    //                     payment_token_residuum.amount,
    //                     managed_biguint!(payment_expected_back_amount)
    //                 );
    //             },
    //         )
    //         .assert_ok();

    //     let final_payment_token_balance =
    //         self.b_mock
    //             .get_esdt_balance(&self.user_address, payment_token_id, 0);
    //     let final_desired_token_balance =
    //         self.b_mock
    //             .get_esdt_balance(&self.user_address, desired_token_id, 0);

    //     assert_eq!(
    //         final_payment_token_balance,
    //         initial_payment_token_balance - &rust_biguint!(payment_amount_max)
    //             + payment_token_swap_amount
    //     );

    //     assert_eq!(
    //         final_desired_token_balance,
    //         initial_desired_token_balance + desired_token_swap_amount
    //     );
    // }
}
