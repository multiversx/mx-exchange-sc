use elrond_wasm::storage::mappers::StorageTokenWrapper;
use elrond_wasm::types::{Address, EsdtLocalRole};
use elrond_wasm_debug::tx_mock::{TxContextStack, TxInputESDT};
use elrond_wasm_debug::{
    managed_biguint, managed_token_id, rust_biguint, testing_framework::*, DebugApi,
};

type RustBigUint = num_bigint::BigUint;

use config::*;
use farm_staking::custom_rewards::{CustomRewardsModule, BLOCKS_IN_YEAR};
use farm_staking::farm_token_merge::StakingFarmTokenAttributes;
use farm_staking::*;
use farm_token::FarmTokenModule;

const FARM_WASM_PATH: &'static str = "farm/output/farm-staking.wasm";

const REWARD_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // reward token ID
const FARMING_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // farming token ID
const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const MIN_FARMING_EPOCHS: u8 = 2;
const MIN_UNBOND_EPOCHS: u64 = 5;
const PENALTY_PERCENT: u64 = 10;
const MAX_APR: u64 = 2_500; // 25%
const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;
const TOTAL_REWARDS_AMOUNT: u64 = 1_000_000_000_000;

const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;

#[allow(dead_code)] // owner_address is unused, at least for now
struct FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub farm_wrapper: ContractObjWrapper<farm_staking::ContractObj<DebugApi>, FarmObjBuilder>,
}

fn setup_farm<FarmObjBuilder>(farm_builder: FarmObjBuilder) -> FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
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
            let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
            let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);

            sc.init(
                farming_token_id,
                division_safety_constant,
                managed_biguint!(MAX_APR),
                MIN_UNBOND_EPOCHS,
            );

            let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
            sc.farm_token().set_token_id(&farm_token_id);

            sc.per_block_reward_amount()
                .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));
            sc.minimum_farming_epochs().set(&MIN_FARMING_EPOCHS);
            sc.penalty_percent().set(&PENALTY_PERCENT);

            sc.state().set(&State::Active);
            sc.produce_rewards_enabled().set(&true);
        })
        .assert_ok();

    blockchain_wrapper.set_esdt_balance(&owner_addr, REWARD_TOKEN_ID, &TOTAL_REWARDS_AMOUNT.into());
    blockchain_wrapper
        .execute_esdt_transfer(
            &owner_addr,
            &farm_wrapper,
            REWARD_TOKEN_ID,
            0,
            &TOTAL_REWARDS_AMOUNT.into(),
            |sc| {
                sc.top_up_rewards();
            },
        )
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
        FARMING_TOKEN_ID,
        &farming_token_roles[..],
    );

    let user_addr = blockchain_wrapper.create_user_account(&rust_biguint!(100_000_000));
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        FARMING_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    FarmSetup {
        blockchain_wrapper,
        owner_address: owner_addr,
        user_address: user_addr,
        farm_wrapper,
    }
}

fn stake_farm<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    farm_in_amount: u64,
    additional_farm_tokens: &[TxInputESDT],
    expected_farm_token_nonce: u64,
    expected_reward_per_share: u64,
    expected_compounded_reward: u64,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
    payments.push(TxInputESDT {
        token_identifier: FARMING_TOKEN_ID.to_vec(),
        nonce: 0,
        value: rust_biguint!(farm_in_amount),
    });
    payments.extend_from_slice(additional_farm_tokens);

    let mut expected_total_out_amount = 0;
    for payment in payments.iter() {
        expected_total_out_amount += payment.value.to_u64_digits()[0];
    }

    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_multi_transfer(
            &farm_setup.user_address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                let payment = sc.stake_farm_endpoint();
                assert_eq!(payment.token_identifier, managed_token_id!(FARM_TOKEN_ID));
                assert_eq!(payment.token_nonce, expected_farm_token_nonce);
                assert_eq!(payment.amount, managed_biguint!(expected_total_out_amount));
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(expected_reward_per_share),
        compounded_reward: managed_biguint!(expected_compounded_reward),
        current_farm_amount: managed_biguint!(expected_total_out_amount),
    };
    b_mock.check_nft_balance(
        &farm_setup.user_address,
        FARM_TOKEN_ID,
        expected_farm_token_nonce,
        &rust_biguint!(expected_total_out_amount),
        Some(&expected_attributes),
    );

    let _ = TxContextStack::static_pop();
}

fn synchronize_farm<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>)
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_tx(
            &farm_setup.owner_address,
            &farm_setup.farm_wrapper,
            &rust_zero,
            |sc| {
                sc.synchronize();
            },
        )
        .assert_ok();
}

fn unbond_farm<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    farm_token_nonce: u64,
    farm_tokem_amount: u64,
    expected_farming_token_out: u64,
    expected_user_farming_token_balance: u64,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            farm_token_nonce,
            &rust_biguint!(farm_tokem_amount),
            |sc| {
                let payment = sc.unbond_farm();
                assert_eq!(
                    payment.token_identifier,
                    managed_token_id!(FARMING_TOKEN_ID)
                );
                assert_eq!(payment.token_nonce, 0);
                assert_eq!(payment.amount, managed_biguint!(expected_farming_token_out));
            },
        )
        .assert_ok();

    b_mock.check_esdt_balance(
        &farm_setup.user_address,
        FARMING_TOKEN_ID,
        &rust_biguint!(expected_user_farming_token_balance),
    );
}

fn unstake_farm<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    farm_token_amount: u64,
    farm_token_nonce: u64,
    expected_rewards_out: u64,
    expected_user_reward_token_balance: &RustBigUint,
    expected_user_farming_token_balance: &RustBigUint,
    expected_new_farm_token_nonce: u64,
    expected_new_farm_token_amount: u64,
    expected_new_farm_token_attributes: &UnbondSftAttributes,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            farm_token_nonce,
            &rust_biguint!(farm_token_amount),
            |sc| {
                let multi_result = sc.unstake_farm();

                let (first_result, second_result) = multi_result.into_tuple();

                assert_eq!(
                    first_result.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(first_result.token_nonce, expected_new_farm_token_nonce);
                assert_eq!(
                    first_result.amount,
                    managed_biguint!(expected_new_farm_token_amount)
                );

                assert_eq!(
                    second_result.token_identifier,
                    managed_token_id!(REWARD_TOKEN_ID)
                );
                assert_eq!(second_result.token_nonce, 0);
                assert_eq!(second_result.amount, managed_biguint!(expected_rewards_out));
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &farm_setup.user_address,
        FARM_TOKEN_ID,
        expected_new_farm_token_nonce,
        &rust_biguint!(expected_new_farm_token_amount),
        Some(expected_new_farm_token_attributes),
    );

    b_mock.check_esdt_balance(
        &farm_setup.user_address,
        REWARD_TOKEN_ID,
        expected_user_reward_token_balance,
    );
    b_mock.check_esdt_balance(
        &farm_setup.user_address,
        FARMING_TOKEN_ID,
        expected_user_farming_token_balance,
    );
}

fn claim_rewards<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    farm_token_amount: u64,
    farm_token_nonce: u64,
    expected_reward_token_out: u64,
    expected_user_reward_token_balance: &RustBigUint,
    expected_user_farming_token_balance: &RustBigUint,
    expected_farm_token_nonce_out: u64,
    expected_reward_per_share: u64,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_esdt_transfer(
            &farm_setup.user_address,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            farm_token_nonce,
            &rust_biguint!(farm_token_amount),
            |sc| {
                let multi_result = sc.claim_rewards();
                let (first_result, second_result) = multi_result.into_tuple();

                assert_eq!(
                    first_result.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(first_result.token_nonce, expected_farm_token_nonce_out);
                assert_eq!(first_result.amount, managed_biguint!(farm_token_amount));

                assert_eq!(
                    second_result.token_identifier,
                    managed_token_id!(REWARD_TOKEN_ID)
                );
                assert_eq!(second_result.token_nonce, 0);
                assert_eq!(
                    second_result.amount,
                    managed_biguint!(expected_reward_token_out)
                );
            },
        )
        .assert_ok();

    let _ = DebugApi::dummy();
    let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
        reward_per_share: managed_biguint!(expected_reward_per_share),
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(farm_token_amount),
    };

    b_mock.check_nft_balance(
        &farm_setup.user_address,
        FARM_TOKEN_ID,
        expected_farm_token_nonce_out,
        &rust_biguint!(farm_token_amount),
        Some(&expected_attributes),
    );
    b_mock.check_esdt_balance(
        &farm_setup.user_address,
        REWARD_TOKEN_ID,
        expected_user_reward_token_balance,
    );
    b_mock.check_esdt_balance(
        &farm_setup.user_address,
        FARMING_TOKEN_ID,
        expected_user_farming_token_balance,
    );

    let _ = TxContextStack::static_pop();
}

fn check_farm_token_supply<FarmObjBuilder>(
    farm_setup: &mut FarmSetup<FarmObjBuilder>,
    expected_farm_token_supply: u64,
) where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let b_mock = &mut farm_setup.blockchain_wrapper;
    b_mock
        .execute_query(&farm_setup.farm_wrapper, |sc| {
            let actual_farm_supply = sc.farm_token_supply().get();
            assert_eq!(
                managed_biguint!(expected_farm_token_supply),
                actual_farm_supply
            );
        })
        .assert_ok();
}

fn set_block_nonce<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>, block_nonce: u64)
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    farm_setup.blockchain_wrapper.set_block_nonce(block_nonce);
}

fn set_block_epoch<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>, block_epoch: u64)
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    farm_setup.blockchain_wrapper.set_block_epoch(block_epoch);
}

#[test]
fn test_staking_setup() {
    let _ = setup_farm(farm_staking::contract_obj);
}

#[test]
fn test_staking_enter_farm() {
    let mut farm_setup = setup_farm(farm_staking::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    stake_farm(
        &mut farm_setup,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);
}

#[test]
fn test_staking_unstake_farm() {
    let mut farm_setup = setup_farm(farm_staking::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    stake_farm(
        &mut farm_setup,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);

    let current_block = 10;
    let current_epoch = 5;
    set_block_epoch(&mut farm_setup, current_epoch);
    set_block_nonce(&mut farm_setup, current_block);
    synchronize_farm(&mut farm_setup);

    let block_diff = current_block - 0;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 40);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    unstake_farm(
        &mut farm_setup,
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        expected_farm_token_nonce + 1,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
        },
    );
    check_farm_token_supply(&mut farm_setup, 0);
}

#[test]
fn test_staking_claim_rewards() {
    let mut farm_setup = setup_farm(farm_staking::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    stake_farm(
        &mut farm_setup,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);

    set_block_epoch(&mut farm_setup, 5);
    set_block_nonce(&mut farm_setup, 10);
    synchronize_farm(&mut farm_setup);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 40;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let expected_reward_per_share = 400_000;
    claim_rewards(
        &mut farm_setup,
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);
}

fn steps_enter_farm_twice<FarmObjBuilder>(farm_builder: FarmObjBuilder) -> FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
{
    let mut farm_setup = setup_farm(farm_builder);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    stake_farm(
        &mut farm_setup,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);

    set_block_epoch(&mut farm_setup, 5);
    set_block_nonce(&mut farm_setup, 10);
    synchronize_farm(&mut farm_setup);

    let second_farm_in_amount = 200_000_000;
    let prev_farm_tokens = [TxInputESDT {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: expected_farm_token_nonce,
        value: rust_biguint!(farm_in_amount),
    }];

    let total_amount = farm_in_amount + second_farm_in_amount;
    let first_reward_share = 0;
    let second_reward_share = 400_000;
    let expected_reward_per_share = (first_reward_share * farm_in_amount
        + second_reward_share * second_farm_in_amount
        + total_amount
        - 1)
        / total_amount;

    stake_farm(
        &mut farm_setup,
        second_farm_in_amount,
        &prev_farm_tokens,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        0,
    );
    check_farm_token_supply(&mut farm_setup, total_amount);

    farm_setup
}

#[test]
fn test_staking_enter_farm_twice() {
    let _ = steps_enter_farm_twice(farm_staking::contract_obj);
}

#[test]
fn test_staking_exit_farm_after_enter_twice() {
    let mut farm_setup = steps_enter_farm_twice(farm_staking::contract_obj);
    let farm_in_amount = 100_000_000;
    let second_farm_in_amount = 200_000_000;

    set_block_epoch(&mut farm_setup, 8);
    set_block_nonce(&mut farm_setup, 25);
    synchronize_farm(&mut farm_setup);

    let _current_farm_supply = farm_in_amount;

    let expected_rewards = 83;
    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount - second_farm_in_amount
            + expected_rewards;
    unstake_farm(
        &mut farm_setup,
        farm_in_amount,
        2,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        3,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: 8 + MIN_UNBOND_EPOCHS,
        },
    );
    check_farm_token_supply(&mut farm_setup, second_farm_in_amount);
}

#[test]
fn test_staking_unbond() {
    let mut farm_setup = setup_farm(farm_staking::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    stake_farm(
        &mut farm_setup,
        farm_in_amount,
        &[],
        expected_farm_token_nonce,
        0,
        0,
    );
    check_farm_token_supply(&mut farm_setup, farm_in_amount);

    let current_block = 10;
    let current_epoch = 5;
    set_block_epoch(&mut farm_setup, current_epoch);
    set_block_nonce(&mut farm_setup, current_block);
    synchronize_farm(&mut farm_setup);

    let block_diff = current_block - 0;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 40);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    unstake_farm(
        &mut farm_setup,
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        expected_farm_token_nonce + 1,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
        },
    );
    check_farm_token_supply(&mut farm_setup, 0);

    set_block_epoch(&mut farm_setup, current_epoch + MIN_UNBOND_EPOCHS);
    synchronize_farm(&mut farm_setup);

    unbond_farm(
        &mut farm_setup,
        expected_farm_token_nonce + 1,
        farm_in_amount,
        farm_in_amount,
        USER_TOTAL_RIDE_TOKENS + expected_rewards,
    );
}
