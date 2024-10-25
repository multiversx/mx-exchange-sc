#![allow(dead_code)]

use common_structs::FarmTokenAttributes;
use multiversx_sc::codec::multi_types::{MultiValue3, OptionalValue};
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::whitebox_legacy::{TxContextStack, TxTokenTransfer};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

type RustBigUint = num_bigint::BigUint;

use config::*;
use farm::*;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_token::FarmTokenModule;
use pair::pair_actions::add_liq::AddLiquidityModule as _;
use pair::{config::ConfigModule as OtherConfigModule, Pair};
use pausable::{PausableModule, State};

pub const FARM_WASM_PATH: &str = "farm/output/farm.wasm";
pub const PAIR_WASM_PATH: &str = "pair/output/pair.wasm";

pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // farming token ID
pub const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MAX_PERCENT: u64 = 10_000;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;
pub const USER_TOTAL_LP_TOKENS: u64 = 5_000_000_000;
pub const MAX_REWARDS_FACTOR: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const LOCKED_TOKEN_ID: &[u8] = b"XMEX-123456";
pub const LOCKED_LP_TOKEN_ID: &[u8] = b"LKLP-123456";
pub const FARM_PROXY_TOKEN_ID: &[u8] = b"PROXY-123456";

pub struct SingleUserFarmSetup<FarmObjBuilder, PairObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

impl<FarmObjBuilder, PairObjBuilder> SingleUserFarmSetup<FarmObjBuilder, PairObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub fn new(farm_builder: FarmObjBuilder, pair_builder: PairObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut blockchain_wrapper = BlockchainStateWrapper::new();
        let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);

        let pair_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            pair_builder,
            PAIR_WASM_PATH,
        );

        // init pair contract
        blockchain_wrapper
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

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        blockchain_wrapper.set_esdt_local_roles(
            pair_wrapper.address_ref(),
            LP_TOKEN_ID,
            &lp_token_roles[..],
        );

        let farm_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            farm_builder,
            FARM_WASM_PATH,
        );

        // init farm contract

        blockchain_wrapper
            .execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
                let farming_token_id = managed_token_id!(LP_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
            })
            .assert_ok();

        blockchain_wrapper
            .execute_tx(&owner_addr, &farm_wrapper, &rust_biguint!(0), |sc| {
                sc.set_boosted_yields_factors(
                    managed_biguint!(MAX_REWARDS_FACTOR),
                    managed_biguint!(USER_REWARDS_ENERGY_CONST),
                    managed_biguint!(USER_REWARDS_FARM_CONST),
                    managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                    managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                );
            })
            .assert_ok();

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARM_TOKEN_ID,
            &farm_token_roles[..],
        );

        let farming_token_roles = [EsdtLocalRole::Burn];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            LP_TOKEN_ID,
            &farming_token_roles[..],
        );

        let reward_token_roles = [EsdtLocalRole::Mint];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            MEX_TOKEN_ID,
            &reward_token_roles[..],
        );

        let user_addr = blockchain_wrapper.create_user_account(&rust_biguint!(100_000_000));
        blockchain_wrapper.set_esdt_balance(
            &user_addr,
            LP_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_LP_TOKENS),
        );

        SingleUserFarmSetup {
            blockchain_wrapper,
            owner_address: owner_addr,
            user_address: user_addr,
            farm_wrapper,
            pair_wrapper,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_liquidity(
        &mut self,
        first_token_amount: u64,
        first_token_min: u64,
        second_token_amount: u64,
        second_token_min: u64,
    ) {
        let payments = vec![
            TxTokenTransfer {
                token_identifier: WEGLD_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxTokenTransfer {
                token_identifier: LOCKED_TOKEN_ID.to_vec(),
                nonce: 0,
                value: rust_biguint!(second_token_amount),
            },
        ];

        self.blockchain_wrapper
            .execute_esdt_multi_transfer(&self.user_address, &self.pair_wrapper, &payments, |sc| {
                let MultiValue3 { 0: payments } = sc.add_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                );

                assert_eq!(payments.0.token_identifier, managed_token_id!(LP_TOKEN_ID));

                assert_eq!(
                    payments.1.token_identifier,
                    managed_token_id!(WEGLD_TOKEN_ID)
                );
                assert_eq!(
                    payments.2.token_identifier,
                    managed_token_id!(LOCKED_TOKEN_ID)
                );
            })
            .assert_ok();
    }

    pub fn enter_farm(
        &mut self,
        farm_in_amount: u64,
        additional_farm_tokens: &[TxTokenTransfer],
        expected_farm_token_nonce: u64,
        expected_reward_per_share: u64,
        expected_entering_epoch: u64,
        expected_compounded_reward: u64,
    ) {
        let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
        payments.push(TxTokenTransfer {
            token_identifier: LP_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farm_in_amount),
        });
        payments.extend_from_slice(additional_farm_tokens);

        let mut expected_total_out_amount = 0;
        for payment in payments.iter() {
            expected_total_out_amount += payment.value.to_u64_digits()[0];
        }

        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_multi_transfer(&self.user_address, &self.farm_wrapper, &payments, |sc| {
                let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
                let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                assert_eq!(
                    out_farm_token.amount,
                    managed_biguint!(expected_total_out_amount)
                );
            })
            .assert_ok();

        DebugApi::dummy();

        let expected_attributes = FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            entering_epoch: expected_entering_epoch,
            compounded_reward: managed_biguint!(expected_compounded_reward),
            current_farm_amount: managed_biguint!(expected_total_out_amount),
            original_owner: managed_address!(&self.user_address),
        };
        b_mock.check_nft_balance(
            &self.user_address,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(expected_total_out_amount),
            Some(&expected_attributes),
        );

        let _ = TxContextStack::static_pop();
    }

    pub fn exit_farm(
        &mut self,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_mex_out: u64,
        expected_farm_token_amount: u64,
        expected_user_mex_balance: &RustBigUint,
        expected_user_lp_token_balance: &RustBigUint,
    ) {
        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.exit_farm_endpoint(OptionalValue::None);

                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(LP_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, 0);
                    assert_eq!(
                        first_result.amount,
                        managed_biguint!(expected_farm_token_amount)
                    );

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(second_result.amount, managed_biguint!(expected_mex_out));
                },
            )
            .assert_ok();

        b_mock.check_esdt_balance(&self.user_address, MEX_TOKEN_ID, expected_user_mex_balance);
        b_mock.check_esdt_balance(
            &self.user_address,
            LP_TOKEN_ID,
            expected_user_lp_token_balance,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn claim_rewards(
        &mut self,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_mex_out: u64,
        expected_user_mex_balance: &RustBigUint,
        expected_user_lp_token_balance: &RustBigUint,
        expected_farm_token_nonce_out: u64,
        expected_reward_per_share: u64,
    ) {
        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.claim_rewards_endpoint(OptionalValue::None);

                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, expected_farm_token_nonce_out);
                    assert_eq!(first_result.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(second_result.amount, managed_biguint!(expected_mex_out));
                },
            )
            .assert_ok();

        DebugApi::dummy();
        let expected_attributes = FarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            entering_epoch: 0,
            compounded_reward: managed_biguint!(0),
            current_farm_amount: managed_biguint!(farm_token_amount),
            original_owner: managed_address!(&self.user_address),
        };

        b_mock.check_nft_balance(
            &self.user_address,
            FARM_TOKEN_ID,
            expected_farm_token_nonce_out,
            &rust_biguint!(farm_token_amount),
            Some(&expected_attributes),
        );
        b_mock.check_esdt_balance(&self.user_address, MEX_TOKEN_ID, expected_user_mex_balance);
        b_mock.check_esdt_balance(
            &self.user_address,
            LP_TOKEN_ID,
            expected_user_lp_token_balance,
        );

        let _ = TxContextStack::static_pop();
    }

    pub fn check_farm_token_supply(&mut self, expected_farm_token_supply: u64) {
        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let actual_farm_supply = sc.farm_token_supply().get();
                assert_eq!(
                    managed_biguint!(expected_farm_token_supply),
                    actual_farm_supply
                );
            })
            .assert_ok();
    }

    pub fn set_block_nonce(&mut self, block_nonce: u64) {
        self.blockchain_wrapper.set_block_nonce(block_nonce);
    }

    pub fn set_block_epoch(&mut self, block_epoch: u64) {
        self.blockchain_wrapper.set_block_epoch(block_epoch);
    }
}
