#![allow(dead_code)]
#![allow(deprecated)]

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, BigUint, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::TxTokenTransfer, whitebox_legacy::*, DebugApi,
};
pub type RustBigUint = num_bigint::BigUint;

use config::*;
use farm::exit_penalty::ExitPenaltyModule;
use farm::*;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};
use rewards::*;

pub const FARM_WASM_PATH: &str = "farm/output/farm.wasm";

pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // farming token ID
pub const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MIN_FARMING_EPOCHS: u64 = 2;
pub const PENALTY_PERCENT: u64 = 10;

pub enum Action {
    EnterFarm(Address, RustBigUint),
    ExitFarm(Address, u64, RustBigUint, RustBigUint),
    RewardPerBlockRateChange(RustBigUint),
}

pub struct Expected {
    reward_reserve: RustBigUint,
    reward_per_share: RustBigUint,
    total_farm_supply: RustBigUint,
}

impl Expected {
    pub fn new(
        reward_reserve: RustBigUint,
        rewards_per_share: RustBigUint,
        total_farm_supply: RustBigUint,
    ) -> Self {
        Self {
            reward_reserve,
            reward_per_share: rewards_per_share,
            total_farm_supply,
        }
    }
}

pub struct FarmRewardsDistrSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
}

impl<FarmObjBuilder> FarmRewardsDistrSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    pub fn new(farm_builder: FarmObjBuilder, per_block_reward_amount: RustBigUint) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut blockchain_wrapper = BlockchainStateWrapper::new();
        let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
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
                let pair_address = managed_address!(&Address::zero());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&to_managed_biguint(per_block_reward_amount));
                sc.minimum_farming_epochs().set(MIN_FARMING_EPOCHS);
                sc.penalty_percent().set(PENALTY_PERCENT);

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
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

        FarmRewardsDistrSetup {
            blockchain_wrapper,
            owner_address: owner_addr,
            farm_wrapper,
        }
    }

    pub fn enter_farm(&mut self, caller: &Address, farm_in_amount: RustBigUint) {
        let payments = vec![TxTokenTransfer {
            token_identifier: LP_TOKEN_ID.to_vec(),
            nonce: 0,
            value: farm_in_amount,
        }];

        let mut expected_total_out_amount = RustBigUint::default();
        for payment in payments.iter() {
            expected_total_out_amount += payment.value.clone();
        }

        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_multi_transfer(caller, &self.farm_wrapper, &payments, |sc| {
                let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
                let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                check_biguint_eq(
                    out_farm_token.amount,
                    expected_total_out_amount,
                    "Enter farm, farm token payment mismatch.",
                );
            })
            .assert_ok();
    }

    pub fn exit_farm(
        &mut self,
        caller: &Address,
        farm_token_nonce: u64,
        farm_out_amount: RustBigUint,
        expected_mex_balance: RustBigUint,
    ) {
        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_transfer(
                caller,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &farm_out_amount.clone(),
                |sc| {
                    let exit_amount = to_managed_biguint(farm_out_amount);
                    let multi_result = sc.exit_farm_endpoint(exit_amount, OptionalValue::None);

                    let (first_result, second_result, _third_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(LP_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, 0);

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                },
            )
            .assert_ok();

        b_mock.check_esdt_balance(caller, MEX_TOKEN_ID, &expected_mex_balance);
    }

    pub fn reward_per_block_rate_change(&mut self, new_rate: RustBigUint) {
        self.blockchain_wrapper
            .execute_tx(
                &self.owner_address,
                &self.farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_per_block_rewards_endpoint(to_managed_biguint(new_rate));
                },
            )
            .assert_ok();
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::EnterFarm(caller, amount) => self.enter_farm(&caller, amount),
            Action::ExitFarm(caller, farm_token_nonce, farm_out_amount, expected_mex_balance) => {
                self.exit_farm(
                    &caller,
                    farm_token_nonce,
                    farm_out_amount,
                    expected_mex_balance,
                )
            }
            Action::RewardPerBlockRateChange(new_rate) => {
                self.reward_per_block_rate_change(new_rate)
            }
        }
    }

    pub fn check_expected(&mut self, expected: Expected) {
        self.blockchain_wrapper
            .execute_query(&self.farm_wrapper, |sc| {
                check_biguint_eq(
                    sc.reward_reserve().get(),
                    expected.reward_reserve,
                    "Reward reserve mismatch.",
                );
                check_biguint_eq(
                    sc.reward_per_share().get(),
                    expected.reward_per_share,
                    "Reward per share mismatch.",
                );
                check_biguint_eq(
                    sc.farm_token_supply().get(),
                    expected.total_farm_supply,
                    "Total farm token supply mismatch.",
                );
            })
            .assert_ok();
    }

    pub fn step(&mut self, block_number: u64, action: Action, expected: Expected) {
        self.blockchain_wrapper.set_block_nonce(block_number + 1); // spreadsheet correction
        self.handle_action(action);
        self.check_expected(expected);
    }

    pub fn new_address_with_lp_tokens(&mut self, amount: RustBigUint) -> Address {
        let blockchain_wrapper = &mut self.blockchain_wrapper;
        let address = blockchain_wrapper.create_user_account(&rust_biguint!(0));
        blockchain_wrapper.set_esdt_balance(&address, LP_TOKEN_ID, &amount);
        address
    }
}

pub fn to_managed_biguint(value: RustBigUint) -> BigUint<DebugApi> {
    BigUint::from_bytes_be(&value.to_bytes_be())
}

pub fn to_rust_biguint(value: BigUint<DebugApi>) -> RustBigUint {
    RustBigUint::from_bytes_be(value.to_bytes_be().as_slice())
}

pub fn check_biguint_eq(actual: BigUint<DebugApi>, expected: RustBigUint, message: &str) {
    assert_eq!(
        actual.clone(),
        to_managed_biguint(expected.clone()),
        "{} Expected: {}, have {}",
        message,
        expected,
        to_rust_biguint(actual),
    );
}
